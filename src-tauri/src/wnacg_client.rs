use std::{io::Cursor, time::Duration};

use anyhow::{anyhow, Context};
use bytes::Bytes;
use image::ImageFormat;
use parking_lot::RwLock;
use reqwest::StatusCode;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, Jitter, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::{
    config::Config,
    types::{Comic, DownloadFormat, GetFavoriteResult, ImgList, SearchResult, UserProfile},
};

const API_DOMAIN: &str = "www.wnacg01.cc";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResp {
    pub ret: bool,
    pub html: String,
}

#[derive(Clone)]
pub struct WnacgClient {
    app: AppHandle,
    api_client: ClientWithMiddleware,
    img_client: ClientWithMiddleware,
}

impl WnacgClient {
    pub fn new(app: AppHandle) -> Self {
        let api_client = create_api_client();
        let img_client = create_img_client();
        Self {
            app,
            api_client,
            img_client,
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<String> {
        let form = json!({
            "login_name": username,
            "login_pass": password,
        });
        // 发送登录请求
        let http_resp = self
            .api_client
            .post(format!("https://{API_DOMAIN}/users-check_login.html"))
            .header("referer", format!("https://{API_DOMAIN}/"))
            .form(&form)
            .send()
            .await?;
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
        let cookie = self.app.state::<RwLock<Config>>().read().cookie.clone();
        // 发送获取用户信息请求
        let http_resp = self
            .api_client
            .get(format!("https://{API_DOMAIN}/users.html"))
            .header("cookie", cookie)
            .header("referer", format!("https://{API_DOMAIN}/"))
            .send()
            .await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        let user_profile = UserProfile::from_html(&body).context("将body解析为UserProfile失败")?;
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
        let http_resp = self
            .api_client
            .get(format!("https://{API_DOMAIN}/search/index.php"))
            .header("referer", format!("https://{API_DOMAIN}/"))
            .query(&params)
            .send()
            .await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        let search_result =
            SearchResult::from_html(&self.app, &body, false).context("将html转换为搜索结果失败")?;
        Ok(search_result)
    }

    pub async fn search_by_tag(
        &self,
        tag_name: &str,
        page_num: i64,
    ) -> anyhow::Result<SearchResult> {
        let url = format!("https://{API_DOMAIN}/albums-index-page-{page_num}-tag-{tag_name}.html");
        let http_resp = self
            .api_client
            .get(url)
            .header("referer", format!("https://{API_DOMAIN}/"))
            .send()
            .await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        let search_result =
            SearchResult::from_html(&self.app, &body, true).context("将html转换为搜索结果失败")?;
        Ok(search_result)
    }

    pub async fn get_img_list(&self, id: i64) -> anyhow::Result<ImgList> {
        let url = format!("https://{API_DOMAIN}/photos-gallery-aid-{id}.html");
        let http_resp = self
            .api_client
            .get(url)
            .header("referer", format!("https://{API_DOMAIN}/"))
            .send()
            .await?;
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
        let img_list =
            serde_json::from_str::<ImgList>(json_str).context("将JSON字符串解析为ImgList失败")?;
        Ok(img_list)
    }

    pub async fn get_comic(&self, id: i64) -> anyhow::Result<Comic> {
        let http_resp = self
            .api_client
            .get(format!("https://{API_DOMAIN}/photos-index-aid-{id}.html"))
            .header("referer", format!("https://{API_DOMAIN}/"))
            .send()
            .await?;
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // TODO: 可以并发获取body和img_list
        let img_list = self.get_img_list(id).await?;
        let comic =
            Comic::from_html(&self.app, &body, img_list).context("将body解析为Comic失败")?;

        Ok(comic)
    }

    pub async fn get_favorite(
        &self,
        shelf_id: i64,
        page_num: i64,
    ) -> anyhow::Result<GetFavoriteResult> {
        let cookie = self.app.state::<RwLock<Config>>().read().cookie.clone();
        // 发送获取收藏夹请求
        let url = format!("https://{API_DOMAIN}/users-users_fav-page-{page_num}-c-{shelf_id}.html");
        let http_resp = self
            .api_client
            .get(url)
            .header("cookie", cookie)
            .header("referer", format!("https://{API_DOMAIN}/"))
            .send()
            .await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 解析html
        let get_favorite_result = GetFavoriteResult::from_html(&self.app, &body)
            .context("将body转换为GetFavoriteResult失败")?;
        Ok(get_favorite_result)
    }

    pub async fn get_img_data_and_format(&self, url: &str) -> anyhow::Result<(Bytes, ImageFormat)> {
        // 发送下载图片请求
        let http_resp = self
            .img_client
            .get(url)
            .header("referer", format!("https://{API_DOMAIN}/"))
            .send()
            .await?;
        // 检查http响应状态码
        let status = http_resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("IP被封，请稍后再试或换条代理线路"));
        } else if status != StatusCode::OK {
            let body = http_resp.text().await?;
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 获取 resp headers 的 content-type 字段
        let content_type = http_resp
            .headers()
            .get("content-type")
            .ok_or(anyhow!("响应中没有content-type字段"))?
            .to_str()
            .context("响应中的content-type字段不是utf-8字符串")?
            .to_string();
        // 获取图片数据
        let image_data = http_resp.bytes().await?;
        // 确定原始图片格式
        let original_format = match content_type.as_str() {
            "image/jpeg" => ImageFormat::Jpeg,
            "image/png" => ImageFormat::Png,
            "image/webp" => ImageFormat::WebP,
            _ => return Err(anyhow!("原图出现了意料之外的格式: {content_type}")),
        };
        // 确定目标格式
        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        let target_format = match download_format {
            DownloadFormat::Jpeg => ImageFormat::Jpeg,
            DownloadFormat::Png => ImageFormat::Png,
            DownloadFormat::Webp => ImageFormat::WebP,
            DownloadFormat::Original => original_format,
        };
        // 如果原始格式与目标格式相同，直接返回
        if original_format == target_format {
            return Ok((image_data, original_format));
        }
        // 否则需要将图片转换为目标格式
        let img =
            image::load_from_memory(&image_data).context("将图片数据转换为DynamicImage失败")?;
        let mut converted_data = Vec::new();
        match target_format {
            ImageFormat::Jpeg => img
                .to_rgb8()
                .write_to(&mut Cursor::new(&mut converted_data), target_format),
            ImageFormat::Png | ImageFormat::WebP => img
                .to_rgba8()
                .write_to(&mut Cursor::new(&mut converted_data), target_format),
            _ => return Err(anyhow!("这里不应该出现目标格式`{target_format:?}`")),
        }
        .context(format!(
            "将`{original_format:?}`转换为`{target_format:?}`失败"
        ))?;

        Ok((Bytes::from(converted_data), target_format))
    }
}

fn create_api_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .base(1) // 指数为1，保证重试间隔为1秒不变
        .jitter(Jitter::Bounded) // 重试间隔在1秒左右波动
        .build_with_total_retry_duration(Duration::from_secs(5)); // 重试总时长为5秒

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .timeout(Duration::from_secs(3)) // 每个请求超过3秒就超时
        .build()
        .unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn create_img_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .build()
        .unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}
