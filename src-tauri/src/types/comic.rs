use std::path::Path;

use anyhow::Context;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::{
    extensions::{AppHandleExt, ToAnyhow},
    utils::filename_filter,
};

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
    // TODO: 拆分成多个函数
    #[allow(clippy::too_many_lines)]
    pub fn from_html(app: &AppHandle, html: &str, img_list: ImgList) -> anyhow::Result<Comic> {
        let document = Html::parse_document(html);

        let document_html = document.html();

        let link = document
            .select(&Selector::parse("head > link").to_anyhow()?)
            .next()
            .context(format!("没有找到漫画id的<link>: {document_html}"))?;
        let link_html = link.html();

        let id = link
            .attr("href")
            .context(format!("漫画id的<link>没有href属性: {link_html}"))?
            .strip_prefix("/feed-index-aid-")
            .context(format!(
                "漫画id的<link>不是以`/feed-index-aid-`开头: {link_html}"
            ))?
            .strip_suffix(".html")
            .context(format!("漫画id的<link>不是以`.html`结尾: {link_html}"))?
            .parse::<i64>()
            .context(format!("漫画id不是整数: {link_html}"))?;

        let h2 = document
            .select(&Selector::parse("#bodywrap > h2").to_anyhow()?)
            .next()
            .context(format!("没有找到漫画标题的<h2>: {document_html}"))?;
        let h2_html = h2.html();

        let title = h2
            .text()
            .next()
            .context(format!("漫画标题的<h2>没有文本: {h2_html}"))?;
        let title = filename_filter(title);

        let img = document
            .select(&Selector::parse(".asTBcell.uwthumb > img").to_anyhow()?)
            .next()
            .context(format!("没有找到封面的<img>: {document_html}"))?;
        let img_html = img.html();

        let cover_src = img
            .attr("src")
            .context(format!("封面的<img>没有src属性: {img_html}"))?
            .trim_start_matches('/')
            .to_string();
        let cover = format!("https://{cover_src}");

        let label = document
            .select(&Selector::parse(".asTBcell.uwconn > label").to_anyhow()?)
            .next()
            .context(format!("没有找到分类的<label>: {document_html}"))?;
        let label_html = label.html();

        let category = label
            .text()
            .next()
            .context(format!("分类的<label>没有文本: {label_html}"))?
            .strip_prefix("分類：")
            .context(format!("分类<label>的文本不是以`分類：`开头: {label_html}"))?
            .to_string();

        let label = document
            .select(&Selector::parse(".asTBcell.uwconn > label").to_anyhow()?)
            .nth(1)
            .context(format!("没有找到图片数量的<label>: {document_html}"))?;
        let label_html = label.html();

        let image_count = label
            .text()
            .next()
            .context(format!("图片数量的<label>没有文本: {label_html}"))?
            .strip_prefix("頁數：")
            .context(format!("图片数量的文本不是以`頁數：`开头: {label_html}"))?
            .strip_suffix("P")
            .context(format!("图片数量的文本不是以`P`结尾: {label_html}"))?
            .parse::<i64>()
            .context(format!("图片数量不是整数: {label_html}"))?;

        let mut tags = vec![];
        let tag_selector = Selector::parse(".tagshow").to_anyhow()?;
        for a in document.select(&tag_selector) {
            let Some(text) = a.text().next() else {
                // 有些标签的<a>没有文本，跳过这些标签
                continue;
            };
            let name = text.trim().to_string();

            let a_html = a.html();
            let href = a
                .attr("href")
                .context(format!("标签的<a>没有href属性: {a_html}"))?
                .to_string();
            // TODO: 这里应该用API_DOMAIN
            let url = format!("https://www.wn01.uk{href}");
            tags.push(Tag { name, url });
        }

        let intro = document
            .select(&Selector::parse(".asTBcell.uwconn > p").to_anyhow()?)
            .next()
            .context(format!("没有找到简介的<p>: {document_html}"))?
            .html();

        let is_downloaded = app.get_config().read().download_dir.join(&title).exists();
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
            "从元数据转为Comic失败，读取元数据文件`{}`失败",
            metadata_path.display()
        ))?;
        let mut comic = serde_json::from_str::<Comic>(&comic_json).context(format!(
            "从元数据转为Comic失败，将`{}`反序列化为Comic失败",
            metadata_path.display()
        ))?;
        // 这个comic中的is_downloaded字段是None，需要重新计算

        let is_downloaded = app
            .get_config()
            .read()
            .download_dir
            .join(&comic.title)
            .exists();
        comic.is_downloaded = Some(is_downloaded);
        Ok(comic)
    }
}
