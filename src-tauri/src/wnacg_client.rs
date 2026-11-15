use std::{
    collections::HashSet,
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
    cookie,
    extensions::{AnyhowErrorToStringChain, AppHandleExt},
    slider::{self, SliderChallenge, SliderVerifyResp},
    types::{Comic, GetShelfResult, ImgList, SearchResult, UserProfile},
};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResp {
    pub ret: bool,
    pub html: String,
}

#[derive(Clone)]
pub struct WnacgClient {
    app: AppHandle,
    api_client: Arc<RwLock<ClientWithMiddleware>>,
    img_client: Arc<RwLock<ClientWithMiddleware>>,
    cover_client: Client,
    /// 解析后的可用镜像域名，避免 301/302 把登录 POST 变成 GET
    resolved_domain: Arc<RwLock<String>>,
}

impl WnacgClient {
    pub fn new(app: AppHandle) -> Self {
        let api_client = create_api_client(&app);
        let api_client = Arc::new(RwLock::new(api_client));

        let img_client = create_img_client(&app);
        let img_client = Arc::new(RwLock::new(img_client));

        let cover_client = create_cover_client(&app);
        Self {
            app,
            api_client,
            img_client,
            cover_client,
            resolved_domain: Arc::new(RwLock::new(String::new())),
        }
    }

    pub fn reload_client(&self) {
        let api_client = create_api_client(&self.app);
        *self.api_client.write() = api_client;
        let img_client = create_img_client(&self.app);
        *self.img_client.write() = img_client;
        // 代理/并发配置变了，顺便重新解析镜像域名
        self.invalidate_domain();
    }

    /// 当前实际可用的域名：先 GET 首页跟随跳转解析出真实域名并缓存
    pub async fn api_domain(&self) -> String {
        let cached = self.resolved_domain.read().clone();
        if !cached.is_empty() {
            return cached;
        }
        let seed = self.app.get_config().read().get_api_domain();
        let domain = match self.probe_domain(&seed).await {
            Ok(domain) => domain,
            Err(err) => {
                let err_title = "解析镜像域名失败，直接使用配置域名";
                let string_chain = err.to_string_chain();
                tracing::warn!(err_title, message = string_chain);
                seed
            }
        };
        *self.resolved_domain.write() = domain.clone();
        domain
    }

    /// 站点换镜像时清掉缓存，下次请求重新解析
    pub fn invalidate_domain(&self) {
        *self.resolved_domain.write() = String::new();
    }

    async fn probe_domain(&self, seed: &str) -> anyhow::Result<String> {
        let client = self.api_client.read().clone();
        let url = format!("https://{seed}/");
        let http_resp = client
            .get(&url)
            .header("user-agent", UA)
            .send()
            .await
            .context("请求站点首页失败")?;
        let final_url = http_resp.url().clone();
        let host = final_url
            .host_str()
            .context("无法从跳转后的地址中解析域名")?
            .to_string();
        tracing::debug!("镜像域名解析完成: {seed} -> {host}");
        Ok(host)
    }

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<String> {
        // 注意：每次滑块验证失败都会累计站点风控次数（10 分钟内失败太多会被临时封），
        // 所以这里不做激进重试：只在镜像跳转/响应格式异常时重来一次。
        let mut retried = false;
        loop {
            let domain = self.api_domain().await;
            // 1. 自动过滑块，拿一次性通行证
            let slider_pass = match self.solve_slider(&domain).await {
                Ok(pass) => pass,
                Err(err) => {
                    let text = err.to_string();
                    if text.contains("失敗次數太多") || text.contains("失败次数太多") {
                        return Err(anyhow!(
                            "滑块验证失败次数过多，站点已临时限制（{text}）。请等 10 分钟后再试，或改用 Cookie 登录"
                        ));
                    }
                    let err_title = "自动完成滑块验证失败，将尝试不带通行证登录";
                    let string_chain = err.to_string_chain();
                    tracing::warn!(err_title, message = string_chain);
                    String::new()
                }
            };
            // 2. 带通行证登录
            let form = json!({
                "login_name": username,
                "login_pass": password,
                "slider_pass": slider_pass,
            });
            let request = self
                .api_client
                .read()
                .post(format!("https://{domain}/index.php?ctl=users&act=check_login"))
                .header("user-agent", UA)
                .header("referer", format!("https://{domain}/users-login.html"))
                .header("origin", format!("https://{domain}"))
                .header("x-requested-with", "XMLHttpRequest")
                .form(&form);
            let http_resp = request.send().await?;

            let status = http_resp.status();
            let headers = http_resp.headers().clone();
            let body = http_resp.text().await?;

            if status.is_redirection() || status == StatusCode::NOT_FOUND {
                // 镜像可能已经换了，清缓存后重试一次
                self.invalidate_domain();
                if retried {
                    return Err(anyhow!("登录接口状态码异常({status}): {body}"));
                }
                retried = true;
                continue;
            }
            if status != StatusCode::OK {
                return Err(anyhow!("预料之外的状态码({status}): {body}"));
            }

            let login_resp = match serde_json::from_str::<LoginResp>(&body) {
                Ok(resp) => resp,
                Err(_) => {
                    if retried {
                        return Err(anyhow!("登录响应不是预期格式: {body}"));
                    }
                    retried = true;
                    continue;
                }
            };

            if !login_resp.ret {
                let msg = login_resp.html.clone();
                if msg.contains("滑塊") || msg.contains("滑块") {
                    return Err(anyhow!(
                        "滑块验证未通过（站点返回: {msg}）。可稍后重试，或改用 Cookie 登录"
                    ));
                }
                return Err(anyhow!("{}", friendly_login_error(&msg)));
            }

            let set_cookie = cookie::from_set_cookie_headers(
                headers
                    .get_all("set-cookie")
                    .iter()
                    .filter_map(|value| value.to_str().ok()),
            );
            if set_cookie.is_empty() {
                return Err(anyhow!("登录成功但响应中没有 set-cookie 字段: {login_resp:?}"));
            }
            tracing::debug!("登录成功，拿到 cookie");
            return Ok(set_cookie);
        }
    }

    /// 走 `index.php?ctl=slider&act=create` + `act=verify`，返回一次性通行证
    pub async fn solve_slider(&self, domain: &str) -> anyhow::Result<String> {
        let create_url = format!(
            "https://{domain}/index.php?ctl=slider&act=create&_t={}",
            slider::now_millis()
        );
        let request = self
            .api_client
            .read()
            .get(create_url)
            .header("user-agent", UA)
            .header("referer", format!("https://{domain}/users-login.html"));
        let http_resp = request.send().await.context("请求滑块题目失败")?;

        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("获取滑块题目失败({status}): {body}"));
        }
        let challenge: SliderChallenge =
            serde_json::from_str(&body).context(format!("解析滑块题目失败: {body}"))?;

        let x = slider::locate_gap(&challenge).context("识别滑块缺口位置失败")?;
        let track = slider::build_track(f64::from(x), u64::from(challenge.w) << 32 | u64::from(x));
        let track_json = serde_json::to_string(&track).context("序列化滑动轨迹失败")?;
        let form = [
            ("token", challenge.token.clone()),
            ("x", format!("{:.1}", f64::from(x))),
            ("track", track_json),
        ];

        let request = self
            .api_client
            .read()
            .post(format!("https://{domain}/index.php?ctl=slider&act=verify"))
            .header("user-agent", UA)
            .header("referer", format!("https://{domain}/users-login.html"))
            .header("origin", format!("https://{domain}"))
            .header("x-requested-with", "XMLHttpRequest")
            .form(&form);
        let http_resp = request.send().await.context("提交滑块验证失败")?;

        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("滑块验证请求失败({status}): {body}"));
        }
        let verify_resp: SliderVerifyResp =
            serde_json::from_str(&body).context(format!("解析滑块验证响应失败: {body}"))?;
        match verify_resp.pass {
            Some(pass) if !pass.is_empty() => {
                tracing::debug!("滑块验证通过，缺口x={x}");
                Ok(pass)
            }
            _ => Err(anyhow!(
                "滑块验证未通过: {}",
                verify_resp.error.unwrap_or_else(|| body.clone())
            )),
        }
    }

    /// 用浏览器导出的 cookie（JSON 数组 / Netscape / 请求头）登录，返回规范化后的 cookie
    pub async fn login_by_cookie(&self, raw_cookie: &str) -> anyhow::Result<String> {
        let cookie = cookie::normalize(raw_cookie);
        if cookie.is_empty() {
            return Err(anyhow!("cookie 为空"));
        }
        let profile = self.get_user_profile_with_cookie(&cookie).await?;
        tracing::debug!("cookie 校验通过，用户: {}", profile.username);
        Ok(cookie)
    }

    pub async fn get_user_profile(&self) -> anyhow::Result<UserProfile> {
        let cookie = cookie::normalize(&self.app.get_config().read().cookie);
        self.get_user_profile_with_cookie(&cookie).await
    }

    pub async fn get_user_profile_with_cookie(&self, cookie: &str) -> anyhow::Result<UserProfile> {
        let api_domain = self.api_domain().await;
        let request = self
            .api_client
            .read()
            .get(format!("https://{api_domain}/users.html"))
            .header("user-agent", UA)
            .header("cookie", cookie)
            .header("referer", format!("https://{api_domain}/"));
        let http_resp = request.send().await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        if body.contains("需要登錄") || body.contains("需要登录") {
            return Err(anyhow!("未登录，cookie 已过期或无效"));
        }
        let user_profile = UserProfile::from_html(&self.app, &body)
            .context(format!("将body解析为UserProfile失败: {body}"))?;
        Ok(user_profile)
    }

    /// 当前保存的登录态（已规范化）
    fn cookie_header(&self) -> String {
        cookie::normalize(&self.app.get_config().read().cookie)
    }

    /// 只有同源的图片/封面请求才带上账号会话：图床走 URL 里的签名参数，
    /// 把 cookie 发过去既没用也多一次泄露面。
    fn same_origin_cookie(&self, url: &str, api_domain: &str) -> Option<String> {
        let target = reqwest::Url::parse(url).ok()?;
        let origin = format!("https://{api_domain}/").parse::<reqwest::Url>().ok()?;
        if target.origin() != origin.origin() {
            return None;
        }
        let cookie = self.cookie_header();
        (!cookie.is_empty()).then_some(cookie)
    }

    /// 站点页面统一走这里：带上登录态，失败信息里带上路径方便排查
    async fn get_site_text(&self, path: &str) -> anyhow::Result<String> {
        let api_domain = self.api_domain().await;
        let url = if path.starts_with("http") {
            path.to_owned()
        } else {
            format!("https://{api_domain}{path}")
        };
        let cookie = self.cookie_header();
        let mut request = self
            .api_client
            .read()
            .get(url)
            .header("user-agent", UA)
            .header("referer", format!("https://{api_domain}/"));
        if !cookie.is_empty() {
            request = request.header("cookie", cookie);
        }
        let http_resp = request.send().await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!(
                "获取页面失败（HTTP {status}，{path}），请检查登录状态和漫画是否存在"
            ));
        }
        Ok(body)
    }

    pub async fn search_by_keyword(
        &self,
        keyword: &str,
        page_num: i64,
    ) -> anyhow::Result<SearchResult> {
        let api_domain = self.api_domain().await;
        let mut url = reqwest::Url::parse(&format!("https://{api_domain}/search/index.php"))
            .context("拼接搜索地址失败")?;
        url.query_pairs_mut()
            .append_pair("q", keyword)
            .append_pair("syn", "yes")
            .append_pair("f", "_all")
            .append_pair("s", "create_time_DESC")
            .append_pair("p", &page_num.to_string());
        let body = self.get_site_text(url.as_str()).await?;
        let search_result = SearchResult::from_html(&self.app, &body, false)
            .context(format!("将html解析为SearchResult失败: {body}"))?;
        Ok(search_result)
    }

    pub async fn search_by_tag(
        &self,
        tag_name: &str,
        page_num: i64,
    ) -> anyhow::Result<SearchResult> {
        let body = self
            .get_site_text(&format!(
                "/albums-index-page-{page_num}-tag-{tag_name}.html"
            ))
            .await?;
        let search_result = SearchResult::from_html(&self.app, &body, true)
            .context(format!("将html解析为SearchResult失败: {body}"))?;
        Ok(search_result)
    }

    pub async fn get_img_list(&self, id: i64) -> anyhow::Result<ImgList> {
        let body = self
            .get_site_text(&format!("/photos-gallery-aid-{id}.html"))
            .await?;
        let img_list_line = body
            .lines()
            .find(|line| line.contains("var imglist = "))
            .context("没有找到包含`imglist`的行")?;
        let start = img_list_line
            .find('[')
            .context("没有在`imglist`行中找到`[`")?;
        let end = img_list_line
            .rfind(']')
            .context("没有在`imglist`行中找到`]`")?;
        let json_str = &img_list_line[start..=end]
            .replace("url:", "\"url\":")
            .replace("caption:", "\"caption\":")
            .replace("fast_img_host+", "")
            .replace("\\\"", "\"");
        let img_list = serde_json::from_str::<ImgList>(json_str)
            .context(format!("将JSON字符串解析为ImgList失败: {json_str}"))?;
        // 每话最后都会塞一张「喜欢本站请收藏」的占位图，合集合并后会变成一话一张
        let mut img_list = img_list;
        img_list.retain(|img| !is_placeholder_image(&img.url));
        if img_list.is_empty() {
            return Err(anyhow!(
                "图片列表为空（漫画ID {id}），请确认登录状态或选择具体章节"
            ));
        }
        Ok(img_list)
    }

    pub async fn get_comic(&self, id: i64) -> anyhow::Result<Comic> {
        let body = self
            .get_site_text(&format!("/photos-index-aid-{id}.html"))
            .await?;
        // 合集（多章节）在详情页里用 `a[data-chid]` 列出每一话，
        // 单话作品直接请求 `photos-gallery-aid-{id}` 就是全部图片
        let img_list = match self.collect_series_chapters(id, &body).await? {
            Some(chapters) => self.get_series_img_list(&chapters).await?,
            None => self.get_img_list(id).await?,
        };
        let comic = Comic::from_html(&self.app, &body, img_list)
            .context(format!("将body和解析为Comic失败: {body}"))?;
        Ok(comic)
    }

    /// 收集合集的所有章节 ID（按站点顺序），不是合集则返回 None
    async fn collect_series_chapters(
        &self,
        id: i64,
        first_page_html: &str,
    ) -> anyhow::Result<Option<Vec<i64>>> {
        let first_page = SeriesPage::parse(first_page_html)?;
        if first_page.chapters.is_empty() {
            return Ok(None);
        }
        let last_page = first_page.last_page();
        let mut chapters = first_page.chapters;
        for page in 2..=last_page {
            let body = self
                .get_site_text(&format!("/photos-index-aid-{id}-page-{page}.html"))
                .await?;
            let mut more = SeriesPage::parse(&body)?.chapters;
            chapters.append(&mut more);
        }
        let mut seen = HashSet::new();
        chapters.retain(|chapter| seen.insert(*chapter));
        tracing::debug!("合集 {id} 共 {} 个章节", chapters.len());
        Ok(Some(chapters))
    }

    /// 按章节顺序合并各章节的图片列表，最多 3 个章节并发
    async fn get_series_img_list(&self, chapters: &[i64]) -> anyhow::Result<ImgList> {
        let mut images = ImgList::default();
        for chunk in chapters.chunks(3) {
            let mut tasks = JoinSet::new();
            for &chapter in chunk {
                let client = self.clone();
                tasks.spawn(async move { (chapter, client.get_img_list(chapter).await) });
            }
            let mut results = Vec::with_capacity(chunk.len());
            while let Some(result) = tasks.join_next().await {
                results.push(result.context("合集章节任务异常")?);
            }
            // 并发会乱序返回，这里按章节在站点上的顺序合并，避免页序错乱
            results.sort_by_key(|(chapter, _)| chunk.iter().position(|c| c == chapter).unwrap_or(0));
            for (chapter, list) in results {
                let list = list.with_context(|| format!("获取合集章节 {chapter} 的图片列表失败"))?;
                images.extend(list.0);
            }
        }
        Ok(images)
    }

    pub async fn get_shelf(&self, shelf_id: i64, page_num: i64) -> anyhow::Result<GetShelfResult> {
        let body = self
            .get_site_text(&format!(
                "/users-users_fav-page-{page_num}-c-{shelf_id}.html"
            ))
            .await?;
        let get_shelf_result = GetShelfResult::from_html(&self.app, &body)
            .context(format!("将body解析为GetShelfResult失败: {body}"))?;
        Ok(get_shelf_result)
    }

    pub async fn get_img_data_and_format(&self, url: &str) -> anyhow::Result<(Bytes, ImageFormat)> {
        let api_domain = self.api_domain().await;
        let mut request = self
            .img_client
            .read()
            .get(url)
            .header("user-agent", UA)
            .header("referer", format!("https://{api_domain}/"));
        if let Some(cookie) = self.same_origin_cookie(url, &api_domain) {
            request = request.header("cookie", cookie);
        }
        let http_resp = request.send().await?;
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
        let api_domain = self.api_domain().await;
        let mut request = self
            .cover_client
            .get(cover_url)
            .header("user-agent", UA)
            .header("referer", format!("https://{api_domain}/"));
        if let Some(cookie) = self.same_origin_cookie(cover_url, &api_domain) {
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
}

/// 站点在每话末尾塞的「喜欢本站请收藏」占位图
fn is_placeholder_image(url: &str) -> bool {
    url.split('?')
        .next()
        .unwrap_or_default()
        .ends_with("shoucang.jpg")
}

/// 详情页里的章节列表信息（合集 / 多话作品）
struct SeriesPage {
    /// 当前页的章节 id（就是各章节的 `photos-gallery-aid-{id}`），按站点顺序
    chapters: Vec<i64>,
    /// 章节总数（`#sr_pub[data-total]`），单话作品没有这个属性，按 0 处理
    total: usize,
    /// 每页章节数（`data-per`），站点默认 12
    per: usize,
}

impl SeriesPage {
    fn parse(html: &str) -> anyhow::Result<SeriesPage> {
        let document = Html::parse_document(html);
        let chapter_selector =
            Selector::parse("a[data-chid]").map_err(|_| anyhow!("章节选择器无效"))?;
        let chapters = document
            .select(&chapter_selector)
            .filter_map(|anchor| anchor.value().attr("data-chid")?.parse::<i64>().ok())
            .collect::<Vec<_>>();

        let mut total = 0usize;
        let mut per = 12usize;
        let box_selector = Selector::parse("[data-total][data-per]")
            .map_err(|_| anyhow!("章节分页选择器无效"))?;
        if let Some(element) = document.select(&box_selector).next() {
            let value = element.value();
            total = value
                .attr("data-total")
                .and_then(|total| total.parse().ok())
                .unwrap_or(0);
            per = value
                .attr("data-per")
                .and_then(|per| per.parse().ok())
                .filter(|per| *per > 0)
                .unwrap_or(12);
        }

        Ok(SeriesPage {
            chapters,
            total,
            per,
        })
    }

    /// 章节列表一共几页（一页 12 话）
    fn last_page(&self) -> usize {
        let total = self.total.max(self.chapters.len());
        if total == 0 {
            return 1;
        }
        total.div_ceil(self.per).max(1)
    }
}

fn friendly_login_error(html: &str) -> String {
    if html.contains("用戶名") || html.contains("用户名") {
        return format!("用户名或密码错误（站点返回: {}）", html.trim());
    }
    if html.contains("滑塊") || html.contains("滑块") {
        return format!("站点要求滑块验证（站点返回: {}）", html.trim());
    }
    format!("登录失败（站点返回: {}）", html.trim())
}

fn create_api_client(app: &AppHandle) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .base(1) // 指数为1，保证重试间隔为1秒不变
        .jitter(Jitter::Bounded) // 重试间隔在1秒左右波动
        .build_with_total_retry_duration(Duration::from_secs(5)); // 重试总时长为5秒

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .user_agent(UA)
        .pool_max_idle_per_host(32)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20)) // 原来是3秒，登录页/搜索页经常超时
        .set_proxy(app, "api_client")
        .build()
        .unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn create_img_client(app: &AppHandle) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    let img_concurrency = app.get_config().read().img_concurrency.max(4);
    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .user_agent(UA)
        .pool_max_idle_per_host(img_concurrency * 2)
        .pool_idle_timeout(Duration::from_secs(120))
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .set_proxy(app, "img_client")
        .build()
        .unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn create_cover_client(app: &AppHandle) -> Client {
    let builder = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .user_agent(UA)
        .pool_max_idle_per_host(16)
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30));
    let builder = match app.get_config().read().proxy_mode {
        ProxyMode::NoProxy => builder.no_proxy(),
        _ => builder,
    };
    builder.build().unwrap_or_else(|_| Client::new())
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

#[cfg(test)]
mod tests {
    use super::{is_placeholder_image, SeriesPage};

    const SERIES_HTML: &str = r#"
<html><body>
  <div class="cc filter sr_filter" id="sr_pub" data-aid="404776" data-page="1"
       data-total="26" data-per="12"
       data-pageurl="/photos-index-aid-404776-page-%5B%23page%23%5D.html">
    <a class="tagshow" data-chid="122260" href="/photos-slide-aid-122260-sid-404776.html">第1話</a>
    <a class="tagshow" data-chid="136893" href="/photos-slide-aid-136893-sid-404776.html">第2話</a>
    <a class="tagshow" data-chid="122256" href="/photos-slide-aid-122256-sid-404776.html">第3話</a>
  </div>
  <a class="tagshow" href="/albums-index-tag-xxx.html">这只是标签，不是章节</a>
</body></html>"#;

    #[test]
    fn series_page_reads_chapters_and_total() {
        let page = SeriesPage::parse(SERIES_HTML).expect("解析章节列表");
        assert_eq!(page.chapters, vec![122260, 136893, 122256]);
        assert_eq!(page.total, 26);
        assert_eq!(page.per, 12);
        // 26 话、每页 12 话 => 3 页
        assert_eq!(page.last_page(), 3);
    }

    #[test]
    fn single_chapter_comic_has_no_chapters() {
        let html = r#"<html><body>
          <div class="addtags">標籤：<a class="tagshow" href="/albums-index-tag-a.html">tag</a></div>
        </body></html>"#;
        let page = SeriesPage::parse(html).expect("解析章节列表");
        assert!(page.chapters.is_empty());
        assert_eq!(page.last_page(), 1);
    }

    #[test]
    fn last_page_falls_back_to_parsed_chapters() {
        // 站点没给 data-total 时，用当前页解析到的章节数兜底
        let html = r#"<html><body>
          <a data-chid="1">a</a><a data-chid="2">b</a><a data-chid="3">c</a>
        </body></html>"#;
        let page = SeriesPage::parse(html).expect("解析章节列表");
        assert_eq!(page.chapters.len(), 3);
        assert_eq!(page.last_page(), 1);
    }

    #[test]
    fn placeholder_image_detection() {
        assert!(is_placeholder_image(
            "//themes/weitu/images/bg/shoucang.jpg"
        ));
        assert!(is_placeholder_image(
            "//themes/weitu/images/bg/shoucang.jpg?verify=1794744000-abc"
        ));
        assert!(!is_placeholder_image(
            "//img3.qy0.ru/data/1222/60/23.jpg?verify=1794744000-abc"
        ));
    }
}
