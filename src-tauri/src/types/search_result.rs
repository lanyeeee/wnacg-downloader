use anyhow::Context;
use parking_lot::RwLock;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::{config::Config, extensions::ToAnyhow, utils::filename_filter};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    comics: Vec<ComicInSearch>,
    current_page: i64,
    total_page: i64,
    is_search_by_tag: bool,
}

impl SearchResult {
    pub fn from_html(
        app: &AppHandle,
        html: &str,
        is_search_by_tag: bool,
    ) -> anyhow::Result<SearchResult> {
        let document = Html::parse_document(html);
        let comic_li_selector = Selector::parse(".li.gallary_item").to_anyhow()?;

        let mut comics = Vec::new();
        for comic_li in document.select(&comic_li_selector) {
            let comic = ComicInSearch::from_li(app, &comic_li)?;
            comics.push(comic);
        }

        let current_page = match document
            .select(&Selector::parse(".thispage").to_anyhow()?)
            .next()
        {
            Some(span) => span
                .text()
                .next()
                .context("没有在当前页码的<span>中找到文本")?
                .parse::<i64>()
                .context("当前页码不是整数")?,
            None => 1,
        };

        let total_page = if is_search_by_tag {
            match document
                .select(&Selector::parse(".f_left.paginator > a").to_anyhow()?)
                .last()
            {
                Some(a) => a
                    .text()
                    .next()
                    .context("没有在最后一页的<a>中找到文本")?
                    .parse::<i64>()
                    .context("最后一页不是整数")?
                    .max(current_page), // 如果是最后一页，那么当前页码就是最后一页
                None => 1,
            }
        } else {
            const PAGE_SIZE: i64 = 24;
            let total = document
                .select(&Selector::parse("#bodywrap .result > b").to_anyhow()?)
                .next()
                .context("没有找到总结果数的<b>")?
                .text()
                .next()
                .context("没有在总结果数的<b>中找到文本")?
                .replace(',', "")
                .parse::<i64>()
                .context("总结果数不是整数")?;
            (total + PAGE_SIZE - 1) / PAGE_SIZE
        };

        Ok(SearchResult {
            comics,
            current_page,
            total_page,
            is_search_by_tag,
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInSearch {
    /// 漫画id
    id: i64,
    /// 漫画标题(带html标签，用于显示匹配关键词)
    title_html: String,
    /// 漫画标题
    title: String,
    /// 封面链接
    cover: String,
    /// 额外信息(209張圖片， 創建於2025-01-05 18:33:19)
    additional_info: String,
    /// 是否已下载
    is_downloaded: bool,
}

impl ComicInSearch {
    pub fn from_li(app: &AppHandle, li: &ElementRef) -> anyhow::Result<ComicInSearch> {
        let title_a = li
            .select(&Selector::parse(".title > a").to_anyhow()?)
            .next()
            .context("没有在<li>中找到标题的<a>")?;

        let id = title_a
            .attr("href")
            .context("没有在标题的<a>中找到href属性")?
            .strip_prefix("/photos-index-aid-")
            .context("href不是以`/photos-index-aid-`开头")?
            .strip_suffix(".html")
            .context("href不是以`.html`结尾")?
            .parse::<i64>()
            .context("id不是整数")?;

        let title_html = title_a
            .attr("title")
            .context("没有在标题的<a>中找到title属性")?
            .trim()
            .to_string();

        let title = title_a.text().collect::<String>();
        let title = filename_filter(&title);

        let cover_src = li
            .select(&Selector::parse("img").to_anyhow()?)
            .next()
            .context("没有在<li>中找到<img>")?
            .attr("src")
            .context("没有在<img>中找到src属性")?;
        let cover = format!("https:{cover_src}");

        let additional_info = li
            .select(&Selector::parse(".info_col").to_anyhow()?)
            .next()
            .context("没有在<li>中找到额外信息的<div>")?
            .text()
            .next()
            .context("没有在额外信息的<div>中找到文本")?
            .trim()
            .to_string();

        let is_downloaded = app
            .state::<RwLock<Config>>()
            .read()
            .download_dir
            .join(&title)
            .exists();

        Ok(ComicInSearch {
            id,
            title_html,
            title,
            cover,
            additional_info,
            is_downloaded,
        })
    }
}
