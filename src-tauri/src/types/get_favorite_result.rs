use anyhow::Context;
use parking_lot::RwLock;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::{config::Config, extensions::ToAnyhow, utils::filename_filter};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetFavoriteResult {
    pub comics: Vec<ComicInFavorite>,
    pub current_page: i64,
    pub total_page: i64,
    pub shelf: Shelf,
    pub shelves: Vec<Shelf>,
}

impl GetFavoriteResult {
    pub fn from_html(app: &AppHandle, html: &str) -> anyhow::Result<GetFavoriteResult> {
        let document = Html::parse_document(html);

        let mut comics = Vec::new();
        for comic_div in document.select(&Selector::parse(".asTB").to_anyhow()?) {
            if let Ok(comic) = ComicInFavorite::from_div(app, &comic_div) {
                comics.push(comic);
            }
        }

        let current_page = match document
            .select(&Selector::parse(".thispage").to_anyhow()?)
            .next()
        {
            Some(span) => {
                let span_html = span.html();
                span.text()
                    .next()
                    .context(format!("没有在当前页码的<span>中找到文本: {span_html}"))?
                    .parse::<i64>()
                    .context(format!("当前页码不是整数: {span_html}"))?
            }
            None => 1,
        };

        let total_page = match document
            .select(&Selector::parse(".f_left.paginator > a").to_anyhow()?)
            .next_back()
        {
            Some(a) => {
                let a_html = a.html();
                a.text()
                    .next()
                    .context(format!("没有在最后一页的<a>中找到文本: {a_html}"))?
                    .parse::<i64>()
                    .context(format!("最后一页不是整数: {a_html}"))?
            }
            .max(current_page), // 如果是最后一页，那么当前页码就是最后一页
            None => 1,
        };

        let shelf = Self::get_shelf(&document)?;

        let shelves = Self::get_shelves(&document)?;

        Ok(GetFavoriteResult {
            comics,
            current_page,
            total_page,
            shelf,
            shelves,
        })
    }

    fn get_shelf(document: &Html) -> anyhow::Result<Shelf> {
        let document_html = document.html();
        let a = document
            .select(&Selector::parse(".cur").to_anyhow()?)
            .next()
            .context(format!("没有找到当前书架的<a>: {document_html}"))?;

        let a_html = a.html();
        let id = a
            .attr("href")
            .context(format!("没有在当前书架的<a>中找到href属性: {a_html}"))?
            .trim()
            .strip_prefix("/users-users_fav-c-")
            .and_then(|s| s.strip_suffix(".html"))
            .unwrap_or("0")
            .parse::<i64>()
            .context(format!("书架id不是整数: {a_html}"))?;

        let name = a
            .text()
            .next()
            .context(format!("没有在当前书架的<a>中找到文本: {a_html}"))?
            .trim()
            .to_string();

        Ok(Shelf { id, name })
    }

    fn get_shelves(document: &Html) -> anyhow::Result<Vec<Shelf>> {
        let mut shelves = Vec::new();
        for a in document.select(&Selector::parse(".nav_list > a").to_anyhow()?) {
            let a_html = a.html();
            let id = a
                .attr("href")
                .context(format!("没有在书架的<a>中找到href属性: {a_html}"))?
                .trim()
                .strip_prefix("/users-users_fav-c-")
                .and_then(|s| s.strip_suffix(".html"))
                .unwrap_or("0")
                .parse::<i64>()
                .context(format!("书架id不是整数: {a_html}"))?;

            let name = a
                .text()
                .next()
                .context(format!("没有在书架的<a>中找到文本: {a_html}"))?
                .trim()
                .to_string();

            shelves.push(Shelf { id, name });
        }

        Ok(shelves)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInFavorite {
    /// 漫画id
    pub id: i64,
    /// 漫画标题
    pub title: String,
    /// 漫画封面链接
    pub cover: String,
    /// 加入收藏的时间
    /// 2025-01-04 16:04:34
    pub favorite_time: String,
    /// 这个漫画属于的书架
    pub shelf: Shelf,
    /// 是否已下载
    pub is_downloaded: bool,
}

impl ComicInFavorite {
    pub fn from_div(app: &AppHandle, div: &ElementRef) -> anyhow::Result<ComicInFavorite> {
        let (id, title) = Self::get_id_and_title(div)?;

        let div_html = div.html();
        let cover_src = div
            .select(&Selector::parse(".asTBcell.thumb img").to_anyhow()?)
            .next()
            .context(format!("没有在漫画的<div>中找到<img>: {div_html}"))?
            .attr("src")
            .context(format!("没有在封面的<img>中找到src属性: {div_html}"))?;
        let cover = format!("https:{cover_src}");

        let favorite_time = div
            .select(&Selector::parse(".l_catg > span").to_anyhow()?)
            .next()
            .context(format!(
                "没有在漫画的<div>中找到收藏时间的<span>: {div_html}"
            ))?
            .text()
            .next()
            .context(format!("没有在标题的<span>中找到文本: {div_html}"))?
            .strip_prefix("創建時間：")
            .context(format!("收藏时间不是以`創建時間：`开头: {div_html}"))?
            .trim()
            .to_string();

        let shelf = Self::get_shelf(div)?;

        let is_downloaded = app
            .state::<RwLock<Config>>()
            .read()
            .download_dir
            .join(&title)
            .exists();

        Ok(ComicInFavorite {
            id,
            title,
            cover,
            favorite_time,
            shelf,
            is_downloaded,
        })
    }

    fn get_id_and_title(div: &ElementRef) -> anyhow::Result<(i64, String)> {
        let div_html = div.html();
        let a = div
            .select(&Selector::parse(".l_title > a").to_anyhow()?)
            .next()
            .context(format!("没有在漫画的<div>中找到标题的<a>: {div_html}"))?;

        let a_html = a.html();
        let id = a
            .attr("href")
            .context(format!("没有在标题的<a>中找到href属性: {a_html}"))?
            .strip_prefix("/photos-index-aid-")
            .context(format!("href不是以`/photos-index-aid-`开头: {a_html}"))?
            .strip_suffix(".html")
            .context(format!("href不是以`.html`结尾: {a_html}"))?
            .parse::<i64>()
            .context(format!("id不是整数: {a_html}"))?;

        let title = a
            .text()
            .next()
            .context(format!("没有在标题的<a>中找到文本: {a_html}"))?
            .trim()
            .to_string();
        let title = filename_filter(&title);

        Ok((id, title))
    }

    fn get_shelf(div: &ElementRef) -> anyhow::Result<Shelf> {
        let div_html = div.html();
        let a = div
            .select(&Selector::parse(".l_catg > a").to_anyhow()?)
            .next()
            .context(format!("没有在漫画的<div>中找到书架的<a>: {div_html}"))?;

        let a_html = a.html();
        let id = a
            .attr("href")
            .context(format!("没有在书架的<a>中找到href属性: {a_html}"))?
            .strip_prefix("/users-users_fav-c-")
            .and_then(|s| s.strip_suffix(".html"))
            .unwrap_or("0")
            .parse::<i64>()
            .context(format!("书架id不是整数: {a_html}"))?;

        let name = a.text().next().unwrap_or_default().trim().to_string();

        Ok(Shelf { id, name })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Shelf {
    /// 书架id
    pub id: i64,
    /// 书架名称
    pub name: String,
}
