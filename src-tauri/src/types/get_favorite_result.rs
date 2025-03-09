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
            let comic = ComicInFavorite::from_div(app, &comic_div)?;
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

        let total_page = match document
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
        let a = document
            .select(&Selector::parse(".cur").to_anyhow()?)
            .next()
            .context("没有找到当前书架的<a>")?;

        let id = a
            .attr("href")
            .context("没有在当前书架的<a>中找到href属性")?
            .trim()
            .strip_prefix("/users-users_fav-c-")
            .and_then(|s| s.strip_suffix(".html"))
            .unwrap_or("0")
            .parse::<i64>()
            .context("书架id不是整数")?;

        let name = a
            .text()
            .next()
            .context("没有在当前书架的<a>中找到文本")?
            .trim()
            .to_string();

        Ok(Shelf { id, name })
    }

    fn get_shelves(document: &Html) -> anyhow::Result<Vec<Shelf>> {
        let mut shelves = Vec::new();
        for a in document.select(&Selector::parse(".nav_list > a").to_anyhow()?) {
            let id = a
                .attr("href")
                .context("没有在书架的<a>中找到href属性")?
                .trim()
                .strip_prefix("/users-users_fav-c-")
                .and_then(|s| s.strip_suffix(".html"))
                .unwrap_or("0")
                .parse::<i64>()
                .context("书架id不是整数")?;

            let name = a
                .text()
                .next()
                .context("没有在书架的<a>中找到文本")?
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

        let cover_src = div
            .select(&Selector::parse(".asTBcell.thumb img").to_anyhow()?)
            .next()
            .context("没有在漫画的<div>中找到<img>")?
            .attr("src")
            .context("没有在封面的<img>中找到src属性")?;
        let cover = format!("https:{cover_src}");

        let favorite_time = div
            .select(&Selector::parse(".l_catg > span").to_anyhow()?)
            .next()
            .context("没有在漫画的<div>中找到收藏时间的<span>")?
            .text()
            .next()
            .context("没有在标题的<span>中找到文本")?
            .strip_prefix("創建時間：")
            .context("收藏时间不是以`創建時間：`开头")?
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
        let a = div
            .select(&Selector::parse(".l_title > a").to_anyhow()?)
            .next()
            .context("没有在漫画的<div>中找到标题的<a>")?;

        let id = a
            .attr("href")
            .context("没有在标题的<a>中找到href属性")?
            .strip_prefix("/photos-index-aid-")
            .context("href不是以`/photos-index-aid-`开头")?
            .strip_suffix(".html")
            .context("href不是以`.html`结尾")?
            .parse::<i64>()
            .context("id不是整数")?;

        let title = a
            .text()
            .next()
            .context("没有在标题的<a>中找到文本")?
            .trim()
            .to_string();
        let title = filename_filter(&title);

        Ok((id, title))
    }

    fn get_shelf(div: &ElementRef) -> anyhow::Result<Shelf> {
        let a = div
            .select(&Selector::parse(".l_catg > a").to_anyhow()?)
            .next()
            .context("没有在漫画的<div>中找到书架的<a>")?;

        let id = a
            .attr("href")
            .context("没有在书架的<a>中找到href属性")?
            .strip_prefix("/users-users_fav-c-")
            .and_then(|s| s.strip_suffix(".html"))
            .unwrap_or("0")
            .parse::<i64>()
            .context("书架id不是整数")?;

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
