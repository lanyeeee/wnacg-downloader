use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context};
use bytes::Bytes;
use image::ImageFormat;
use parking_lot::RwLock;
use reqwest::{Client, StatusCode};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, Jitter, RetryTransientMiddleware};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;
use tokio::task::JoinSet;

use crate::{
    config::ProxyMode,
    extensions::{AnyhowErrorToStringChain, AppHandleExt},
    types::{Comic, GetShelfResult, ImgList, SearchResult, UserProfile},
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResp {
    pub ret: bool,
    pub html: String,
}

pub enum LoginOutcome {
    Authenticated(String),
    CaptchaRequired,
}

#[derive(Clone)]
pub struct WnacgClient {
    app: AppHandle,
    api_client: Arc<RwLock<ClientWithMiddleware>>,
    img_client: Arc<RwLock<ClientWithMiddleware>>,
    cover_client: Client,
}

impl WnacgClient {
    pub fn new(app: AppHandle) -> Self {
        let api_client = create_api_client(&app);
        let api_client = Arc::new(RwLock::new(api_client));

        let img_client = create_img_client(&app);
        let img_client = Arc::new(RwLock::new(img_client));

        let cover_client = Client::new();
        Self {
            app,
            api_client,
            img_client,
            cover_client,
        }
    }

    pub fn reload_client(&self) {
        let api_client = create_api_client(&self.app);
        *self.api_client.write() = api_client;
        let img_client = create_img_client(&self.app);
        *self.img_client.write() = img_client;
    }

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<LoginOutcome> {
        self.login_request(&self.get_api_domain(), username, password, None, "")
            .await
    }

    pub async fn login_with_captcha(
        &self,
        api_domain: &str,
        username: &str,
        password: &str,
        pass: &str,
        browser_cookie: &str,
    ) -> anyhow::Result<LoginOutcome> {
        self.login_request(api_domain, username, password, Some(pass), browser_cookie)
            .await
    }

    async fn login_request(
        &self,
        api_domain: &str,
        username: &str,
        password: &str,
        pass: Option<&str>,
        browser_cookie: &str,
    ) -> anyhow::Result<LoginOutcome> {
        let mut form = json!({
            "login_name": username,
            "login_pass": password,
        });
        if let Some(pass) = pass {
            form["slider_pass"] = json!(pass);
        }
        let mut request = self
            .api_client
            .read()
            .post(format!("https://{api_domain}/users-check_login.html"))
            .header("referer", format!("https://{api_domain}/"))
            .form(&form);
        if !browser_cookie.is_empty() {
            request = request.header("cookie", browser_cookie);
        }
        let http_resp = request.send().await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let headers = http_resp.headers().clone();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("登录接口返回状态码 {status}"));
        }
        // 尝试将body解析为LoginResp
        let login_resp =
            serde_json::from_str::<LoginResp>(&body).context("登录接口返回了无法识别的响应")?;
        // 检查LoginResp的ret字段，如果为false则登录失败
        if !login_resp.ret {
            if login_resp.html.contains("滑塊") || login_resp.html.contains("滑块") {
                return Ok(LoginOutcome::CaptchaRequired);
            }
            return Err(anyhow!("{}", login_resp.html));
        }
        let mut cookies = BTreeMap::new();
        for pair in browser_cookie.split(';') {
            if let Some((name, value)) = pair.trim().split_once('=') {
                cookies.insert(name.to_owned(), value.to_owned());
            }
        }
        for header in headers.get_all("set-cookie") {
            let header = header.to_str().context("登录会话响应无效")?;
            if let Some((name, value)) = header
                .split(';')
                .next()
                .and_then(|pair| pair.split_once('='))
            {
                cookies.insert(name.trim().to_owned(), value.to_owned());
            }
        }
        if cookies.is_empty() {
            return Err(anyhow!("登录成功，但服务器未返回登录会话"));
        }
        Ok(LoginOutcome::Authenticated(
            cookies
                .into_iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }

    pub async fn get_user_profile(&self) -> anyhow::Result<UserProfile> {
        let cookie = self.app.get_config().read().cookie.clone();
        let api_domain = self.get_api_domain();
        self.get_user_profile_with_cookie(&api_domain, &cookie)
            .await
    }

    pub async fn get_user_profile_with_cookie(
        &self,
        api_domain: &str,
        cookie: &str,
    ) -> anyhow::Result<UserProfile> {
        // Validate a candidate session without changing the active account.
        let request = self
            .api_client
            .read()
            .get(format!("https://{api_domain}/users.html"))
            .header("cookie", cookie)
            .header("referer", format!("https://{api_domain}/"));
        let http_resp = request.send().await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为UserProfile
        let user_profile = UserProfile::from_html(&self.app, &body)?;
        Ok(user_profile)
    }

    pub async fn search_by_keyword(
        &self,
        keyword: &str,
        page_num: i64,
    ) -> anyhow::Result<SearchResult> {
        let params = json!({
            "q": keyword,
            "syn": "yes",
            "f": "_all",
            "s": "create_time_DESC",
            "p": page_num,
        });
        let api_domain = self.get_api_domain();
        let cookie = self.app.get_config().read().cookie.clone();
        let request = self
            .api_client
            .read()
            .get(format!("https://{api_domain}/search/index.php"))
            .header("cookie", cookie)
            .header("referer", format!("https://{api_domain}/"))
            .query(&params);
        let http_resp = request.send().await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为SearchResult
        let search_result = SearchResult::from_html(&self.app, &body, false)
            .context(format!("将html解析为SearchResult失败: {body}"))?;
        Ok(search_result)
    }

    pub async fn search_by_tag(
        &self,
        tag_name: &str,
        page_num: i64,
    ) -> anyhow::Result<SearchResult> {
        let api_domain = self.get_api_domain();
        let cookie = self.app.get_config().read().cookie.clone();
        let url = format!("https://{api_domain}/albums-index-page-{page_num}-tag-{tag_name}.html");
        let request = self
            .api_client
            .read()
            .get(url)
            .header("cookie", cookie)
            .header("referer", format!("https://{api_domain}/"));
        let http_resp = request.send().await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为SearchResult
        let search_result = SearchResult::from_html(&self.app, &body, true)
            .context(format!("将html解析为SearchResult失败: {body}"))?;
        Ok(search_result)
    }

    pub async fn get_img_list(&self, id: i64) -> anyhow::Result<ImgList> {
        let body = self
            .get_site_text(&format!("/photos-gallery-aid-{id}.html"))
            .await?;
        // 找到包含`imglist`的行
        let img_list_line = body
            .lines()
            .find(|line| line.contains("var imglist = "))
            .context("没有找到包含`imglist`的行")?;
        // 找到`imglist`行中的 JSON 部分的起始和结束位置
        let start = img_list_line
            .find('[')
            .context("没有在`imglist`行中找到`[`")?;
        let end = img_list_line
            .rfind(']')
            .context("没有在`imglist`行中找到`]`")?;
        // 将 JSON 部分提取出来，并转为合法的 JSON 字符串
        let json_str = &img_list_line[start..=end]
            .replace("url:", "\"url\":")
            .replace("caption:", "\"caption\":")
            .replace("fast_img_host+", "")
            .replace("\\\"", "\"");
        // 将 JSON 字符串解析为 ImgList
        let mut img_list =
            serde_json::from_str::<ImgList>(json_str).context("网站返回的图片列表格式无法解析")?;
        img_list.retain(|img| {
            !img.url
                .split('?')
                .next()
                .unwrap_or("")
                .ends_with("shoucang.jpg")
        });
        if img_list.is_empty() {
            return Err(anyhow!("图片列表为空，请确认登录状态或选择具体章节"));
        }
        let origin: reqwest::Url = format!("https://{}/", self.get_api_domain()).parse()?;
        for img in img_list.iter_mut() {
            img.url = origin.join(&img.url)?.to_string();
        }
        Ok(img_list)
    }

    pub async fn get_comic(&self, id: i64) -> anyhow::Result<Comic> {
        let body = self
            .get_site_text(&format!("/photos-index-aid-{id}.html"))
            .await?;
        let (chapters, last_page) = series_chapters(&body, id)?;
        let img_list = if chapters.is_empty() {
            self.get_img_list(id).await?
        } else {
            self.get_series_img_list(id, chapters, last_page).await?
        };
        // 尝试将body解析为Comic
        let comic = Comic::from_html(&self.app, &body, img_list).context("无法解析漫画详情")?;

        Ok(comic)
    }

    async fn get_site_text(&self, path: &str) -> anyhow::Result<String> {
        let api_domain = self.get_api_domain();
        let cookie = self.app.get_config().read().cookie.clone();
        let request = self
            .api_client
            .read()
            .get(format!("https://{api_domain}{path}"))
            .header("cookie", cookie)
            .header("referer", format!("https://{api_domain}/"));
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!(
                "获取漫画页面失败（HTTP {status}，{path}），请检查登录状态和漫画是否存在"
            ));
        }
        Ok(response.text().await?)
    }

    async fn get_series_img_list(
        &self,
        id: i64,
        mut chapters: Vec<i64>,
        last_page: i64,
    ) -> anyhow::Result<ImgList> {
        for page in 2..=last_page {
            let body = self
                .get_site_text(&format!("/photos-index-aid-{id}-page-{page}.html"))
                .await?;
            chapters.extend(series_chapters(&body, id)?.0);
        }
        let mut seen = HashSet::new();
        chapters.retain(|chapter| seen.insert(*chapter));
        let mut lists = vec![None; chapters.len()];
        let mut pending = chapters.into_iter().enumerate();
        let mut tasks = JoinSet::new();
        loop {
            while tasks.len() < 3 {
                let Some((index, chapter)) = pending.next() else {
                    break;
                };
                let client = self.clone();
                tasks.spawn(async move {
                    let list = client
                        .get_img_list(chapter)
                        .await
                        .with_context(|| format!("获取合集章节 {chapter} 的图片列表失败"))?;
                    Ok::<_, anyhow::Error>((index, list))
                });
            }
            let Some(result) = tasks.join_next().await else {
                break;
            };
            let (index, list) = result.context("加载合集章节失败")??;
            lists[index] = Some(list);
        }
        let mut images = ImgList::default();
        for list in lists {
            images.extend(list.context("合集章节未加载完整")?.0);
        }
        Ok(images)
    }

    pub async fn get_shelf(&self, shelf_id: i64, page_num: i64) -> anyhow::Result<GetShelfResult> {
        let cookie = self.app.get_config().read().cookie.clone();
        // 发送获取书架请求
        let api_domain = self.get_api_domain();
        let url = format!("https://{api_domain}/users-users_fav-page-{page_num}-c-{shelf_id}.html");
        let request = self
            .api_client
            .read()
            .get(url)
            .header("cookie", cookie)
            .header("referer", format!("https://{api_domain}/"));
        let http_resp = request.send().await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为GetShelfResult
        let get_shelf_result = GetShelfResult::from_html(&self.app, &body)
            .context(format!("将body解析为GetShelfResult失败: {body}"))?;
        Ok(get_shelf_result)
    }

    pub async fn get_img_data_and_format(&self, url: &str) -> anyhow::Result<(Bytes, ImageFormat)> {
        // 发送下载图片请
        let api_domain = self.get_api_domain();
        let mut request = self
            .img_client
            .read()
            .get(url)
            .header("referer", format!("https://{api_domain}/"));
        if let Some(cookie) = self.site_cookie_for_url(url) {
            request = request.header("cookie", cookie);
        }
        let http_resp = request.send().await?;
        // 检查http响应状态码
        let status = http_resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("IP被封，请在配置中减少并发数或设置下载完成后的休息时间，以此降低下载速度，稍后再试"));
        } else if status != StatusCode::OK {
            let body = http_resp.text().await?;
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        let image_data = http_resp.bytes().await?;

        let format = image::guess_format(&image_data)
            .context("无法从图片数据中猜测出图片格式，可能图片数据不完整或已损坏")?;

        Ok((image_data, format))
    }

    pub async fn get_cover_data(&self, cover_url: &str) -> anyhow::Result<Bytes> {
        let api_domain = self.get_api_domain();
        let mut request = self
            .cover_client
            .get(cover_url)
            .header("referer", format!("https://{api_domain}/"));
        if let Some(cookie) = self.site_cookie_for_url(cover_url) {
            request = request.header("cookie", cookie);
        }
        let http_resp = request.send().await?;
        let status = http_resp.status();
        if status != StatusCode::OK {
            let body = http_resp.text().await?;
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        let cover_data = http_resp.bytes().await?;
        Ok(cover_data)
    }

    fn get_api_domain(&self) -> String {
        self.app.get_config().read().get_api_domain()
    }

    fn site_cookie_for_url(&self, url: &str) -> Option<String> {
        let config = self.app.get_config();
        let config = config.read();
        let origin: reqwest::Url = format!("https://{}/", config.get_api_domain())
            .parse()
            .ok()?;
        let target: reqwest::Url = url.parse().ok()?;
        // Image hosts use signed URLs; send the account session only to its origin.
        (target.origin() == origin.origin() && !config.cookie.is_empty())
            .then(|| config.cookie.clone())
    }
}

fn series_chapters(html: &str, id: i64) -> anyhow::Result<(Vec<i64>, i64)> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(".tagshow[data-chid]").map_err(|_| anyhow!("章节选择器无效"))?;
    let chapters = document
        .select(&selector)
        .filter_map(|a| a.attr("data-chid").and_then(|id| id.parse().ok()))
        .collect();
    let page_prefix = format!("/photos-index-aid-{id}-page-");
    let links = Selector::parse("a[href]").map_err(|_| anyhow!("分页选择器无效"))?;
    let last_page = document
        .select(&links)
        .filter_map(|a| {
            a.attr("href")?
                .strip_prefix(&page_prefix)?
                .strip_suffix(".html")?
                .parse::<i64>()
                .ok()
        })
        .max()
        .unwrap_or(1);
    Ok((chapters, last_page))
}

fn create_api_client(app: &AppHandle) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .base(1) // 指数为1，保证重试间隔为1秒不变
        .jitter(Jitter::Bounded) // 重试间隔在1秒左右波动
        .build_with_total_retry_duration(Duration::from_secs(5)); // 重试总时长为5秒

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .set_proxy(app, "api_client")
        .build()
        .unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn create_img_client(app: &AppHandle) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .set_proxy(app, "img_client")
        .build()
        .unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

trait ClientBuilderExt {
    fn set_proxy(self, app: &AppHandle, client_name: &str) -> Self;
}

impl ClientBuilderExt for reqwest::ClientBuilder {
    fn set_proxy(self, app: &AppHandle, client_name: &str) -> reqwest::ClientBuilder {
        let proxy_mode = app.get_config().read().proxy_mode;
        match proxy_mode {
            ProxyMode::System => self,
            ProxyMode::NoProxy => self.no_proxy(),
            ProxyMode::Custom => {
                let config = app.get_config().inner().read();
                let proxy_host = &config.proxy_host;
                let proxy_port = &config.proxy_port;
                let proxy_url = format!("http://{proxy_host}:{proxy_port}");

                match reqwest::Proxy::all(&proxy_url).map_err(anyhow::Error::from) {
                    Ok(proxy) => self.proxy(proxy),
                    Err(err) => {
                        let err_title = format!("{client_name}将`{proxy_url}`设为代理失败，将直连");
                        let string_chain = err.to_string_chain();
                        tracing::error!(err_title, message = string_chain);
                        self.no_proxy()
                    }
                }
            }
        }
    }
}
