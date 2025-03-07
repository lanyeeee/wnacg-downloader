use anyhow::{anyhow, Context};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::extensions::ToAnyhow;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    /// 用户名
    pub username: String,
    /// 头像url
    pub avatar: String,
}
impl UserProfile {
    pub fn from_html(html: &str) -> anyhow::Result<UserProfile> {
        // 解析html
        let document = Html::parse_document(html);
        // 检查是否登录，如果有`.title.title_c`则未登录
        let is_login = document
            .select(&Selector::parse(".title.title_c").to_anyhow()?)
            .next()
            .is_none();
        if !is_login {
            return Err(anyhow!("未登录，cookie已过期或cookie无效"));
        }
        // 获取头像与用户名的<a>
        let a = document
            .select(&Selector::parse(".top_utab.ui > a").to_anyhow()?)
            .next()
            .context("没有找到头像与用户名的<a>")?;
        // 获取头像url
        let avatar = a
            .select(&Selector::parse("img").to_anyhow()?)
            .next()
            .context("没有在头像与用户名的<a>中找到<img>")?
            .attr("src")
            .map_or("https://www.wn01.uk/userpic/nopic.png".to_string(), |src| {
                format!("https://www.wn01.uk/{src}")
            });
        // 获取用户名
        let username = a
            .text()
            .next()
            .context("没有找到用户名相关的文本")?
            .trim()
            .to_string();

        let user_profile = UserProfile { username, avatar };
        Ok(user_profile)
    }
}
