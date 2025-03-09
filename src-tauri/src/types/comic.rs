use std::path::Path;

use anyhow::Context;
use parking_lot::RwLock;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::{config::Config, extensions::ToAnyhow, utils::filename_filter};

use super::{ImgList, Tag};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Comic {
    /// 漫画id
    pub id: i64,
    /// 漫画标题
    pub title: String,
    /// 封面链接
    pub cover: String,
    /// 分类
    pub category: String,
    /// 漫画有多少张图片
    pub image_count: i64,
    /// 标签
    pub tags: Vec<Tag>,
    /// 简介
    pub intro: String,
    /// 是否已下载
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_downloaded: Option<bool>,
    /// 图片列表
    pub img_list: ImgList,
}

impl Comic {
    pub fn from_html(app: &AppHandle, html: &str, img_list: ImgList) -> anyhow::Result<Comic> {
        let document = Html::parse_document(html);

        let id = document
            .select(&Selector::parse("head > link").to_anyhow()?)
            .next()
            .context("没有找到漫画id的<link>")?
            .attr("href")
            .context("漫画id的<link>没有href属性")?
            .strip_prefix("/feed-index-aid-")
            .context("漫画id的<link>不是以`/feed-index-aid-`开头")?
            .strip_suffix(".html")
            .context("漫画id的<link>不是以`.html`结尾")?
            .parse::<i64>()
            .context("漫画id不是整数")?;

        let title = document
            .select(&Selector::parse("#bodywrap > h2").to_anyhow()?)
            .next()
            .context("没有找到漫画标题的<h2>")?
            .text()
            .next()
            .context("漫画标题的<h2>没有文本")?;
        let title = filename_filter(title);

        let cover_src = document
            .select(&Selector::parse(".asTBcell.uwthumb > img").to_anyhow()?)
            .next()
            .context("没有找到封面的<img>")?
            .attr("src")
            .context("封面的<img>没有src属性")?
            .trim_start_matches('/')
            .to_string();
        let cover = format!("https://{cover_src}");

        let category = document
            .select(&Selector::parse(".asTBcell.uwconn > label").to_anyhow()?)
            .next()
            .context("没有找到分类的<label>")?
            .text()
            .next()
            .context("分类的<label>没有文本")?
            .strip_prefix("分類：")
            .context("分类<label>的文本不是以`分類：`开头")?
            .to_string();

        let image_count = document
            .select(&Selector::parse(".asTBcell.uwconn > label").to_anyhow()?)
            .nth(1)
            .context("没有找到图片数量的<label>")?
            .text()
            .next()
            .context("图片数量的<label>没有文本")?
            .strip_prefix("頁數：")
            .context("图片数量的文本不是以`頁數：`开头")?
            .strip_suffix("P")
            .context("图片数量的文本不是以`P`结尾")?
            .parse::<i64>()
            .context("图片数量不是整数")?;

        let mut tags = vec![];
        let tag_selector = Selector::parse(".tagshow").to_anyhow()?;
        for a in document.select(&tag_selector) {
            let Some(text) = a.text().next() else {
                // 有些标签的<a>没有文本，跳过这些标签
                continue;
            };
            let name = text.trim().to_string();

            let href = a.attr("href").context("标签的<a>没有href属性")?.to_string();
            let url = format!("https://www.wn01.uk{href}");
            tags.push(Tag { name, url });
        }

        let intro = document
            .select(&Selector::parse(".asTBcell.uwconn > p").to_anyhow()?)
            .next()
            .context("没有找到简介的<p>")?
            .html();

        let is_downloaded = app
            .state::<RwLock<Config>>()
            .read()
            .download_dir
            .join(&title)
            .exists();
        let is_downloaded = Some(is_downloaded);

        Ok(Comic {
            id,
            title,
            cover,
            category,
            image_count,
            tags,
            intro,
            is_downloaded,
            img_list,
        })
    }

    pub fn from_metadata(app: &AppHandle, metadata_path: &Path) -> anyhow::Result<Comic> {
        let comic_json = std::fs::read_to_string(metadata_path).context(format!(
            "从元数据转为Comic失败，读取元数据文件 {metadata_path:?} 失败"
        ))?;
        let mut comic = serde_json::from_str::<Comic>(&comic_json).context(format!(
            "从元数据转为Comic失败，将 {metadata_path:?} 反序列化为Comic失败"
        ))?;
        // 这个comic中的is_downloaded字段是None，需要重新计算

        let is_downloaded = app
            .state::<RwLock<Config>>()
            .read()
            .download_dir
            .join(&comic.title)
            .exists();
        comic.is_downloaded = Some(is_downloaded);
        Ok(comic)
    }
}
