use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Context};
use bytes::Bytes;
use image::ImageFormat;
use parking_lot::RwLock;
use reqwest::{Client, StatusCode};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, Jitter, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

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

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<String> {
        let form = json!({
            "login_name": username,
            "login_pass": password,
        });
        // 发送登录请求
        let api_domain = self.get_api_domain();
        let request = self
            .api_client
            .read()
            .post(format!("https://{api_domain}/users-check_login.html"))
            .header("referer", format!("https://{api_domain}/"))
            .form(&form);
        let http_resp = request.send().await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let headers = http_resp.headers().clone();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为LoginResp
        let login_resp = serde_json::from_str::<LoginResp>(&body)
            .context(format!("将body解析为LoginResp失败: {body}"))?;
        // 检查LoginResp的ret字段，如果为false则登录失败
        if !login_resp.ret {
            return Err(anyhow!("登录失败: {login_resp:?}"));
        }
        // 获取resp header中的set-cookie字段
        let cookie = headers
            .get("set-cookie")
            .ok_or(anyhow!("响应中没有set-cookie字段: {login_resp:?}"))?
            .to_str()
            .context(format!(
                "响应中的set-cookie字段不是utf-8字符串: {login_resp:?}"
            ))?
            .to_string();

        Ok(cookie)
    }

    pub async fn get_user_profile(&self) -> anyhow::Result<UserProfile> {
        let cookie = self.app.get_config().read().cookie.clone();
        // 发送获取用户信息请求
        let api_domain = self.get_api_domain();
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
        let user_profile = UserProfile::from_html(&self.app, &body)
            .context(format!("将body解析为UserProfile失败: {body}"))?;
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
        let request = self
            .api_client
            .read()
            .get(format!("https://{api_domain}/search/index.php"))
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
        let url = format!("https://{api_domain}/albums-index-page-{page_num}-tag-{tag_name}.html");
        let request = self
            .api_client
            .read()
            .get(url)
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
        let api_domain = self.get_api_domain();
        let url = format!("https://{api_domain}/photos-gallery-aid-{id}.html");
        let request = self
            .api_client
            .read()
            .get(url)
            .header("referer", format!("https://{api_domain}/"));
        let http_resp = request.send().await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
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
        let img_list = serde_json::from_str::<ImgList>(json_str)
            .context(format!("将JSON字符串解析为ImgList失败: {json_str}"))?;
        Ok(img_list)
    }

    pub async fn get_comic(&self, id: i64) -> anyhow::Result<Comic> {
        let api_domain = self.get_api_domain();
        let request = self
            .api_client
            .read()
            .get(format!("https://{api_domain}/photos-index-aid-{id}.html"))
            .header("referer", format!("https://{api_domain}/"));
        let http_resp = request.send().await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // TODO: 可以并发获取body和img_list
        let img_list = self.get_img_list(id).await?;
        // 尝试将body解析为Comic
        let comic = Comic::from_html(&self.app, &body, img_list)
            .context(format!("将body和解析为Comic失败: {body}"))?;

        Ok(comic)
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
        let request = self
            .img_client
            .read()
            .get(url)
            .header("referer", format!("https://{api_domain}/"));
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
        let http_resp = self
            .cover_client
            .get(cover_url)
            .header("referer", format!("https://{api_domain}/"))
            .send()
            .await?;
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
}

fn create_api_client(app: &AppHandle) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .base(1) // 指数为1，保证重试间隔为1秒不变
        .jitter(Jitter::Bounded) // 重试间隔在1秒左右波动
        .build_with_total_retry_duration(Duration::from_secs(5)); // 重试总时长为5秒

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .timeout(Duration::from_secs(3)) // 每个请求超过3秒就超时
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
