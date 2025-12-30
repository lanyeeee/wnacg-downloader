use std::time::Duration;

use anyhow::Context;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;
use tokio::{task::JoinSet, time::sleep};

use crate::{
    config::Config,
    errors::{CommandError, CommandResult},
    events::DownloadShelfEvent,
    export,
    extensions::{AnyhowErrorToStringChain, AppHandleExt},
    logger,
    types::{Comic, GetShelfResult, SearchResult, UserProfile},
};

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_config(app: AppHandle) -> Config {
    let config = app.get_config();
    let config = config.read().clone();
    tracing::debug!("获取配置成功");
    config
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn save_config(app: AppHandle, config: Config) -> CommandResult<()> {
    let config_state = app.get_config();
    let wnacg_client = app.get_wnacg_client();

    let proxy_changed = {
        let config_state = config_state.read();
        config_state.proxy_mode != config.proxy_mode
            || config_state.proxy_host != config.proxy_host
            || config_state.proxy_port != config.proxy_port
    };

    let enable_file_logger = config.enable_file_logger;
    let file_logger_changed = config_state.read().enable_file_logger != enable_file_logger;

    {
        // 包裹在大括号中，以便自动释放写锁
        let mut config_state = config_state.write();
        *config_state = config;
        config_state
            .save(&app)
            .map_err(|err| CommandError::from("保存配置失败", err))?;
        tracing::debug!("保存配置成功");
    }

    if proxy_changed {
        wnacg_client.reload_client();
    }

    if file_logger_changed {
        if enable_file_logger {
            logger::reload_file_logger()
                .map_err(|err| CommandError::from("重新加载文件日志失败", err))?;
        } else {
            logger::disable_file_logger()
                .map_err(|err| CommandError::from("禁用文件日志失败", err))?;
        }
    }

    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn login(app: AppHandle, username: String, password: String) -> CommandResult<String> {
    let wnacg_client = app.get_wnacg_client();

    let cookie = wnacg_client
        .login(&username, &password)
        .await
        .map_err(|err| CommandError::from("登录失败", err))?;
    tracing::debug!("登录成功");
    Ok(cookie)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_user_profile(app: AppHandle) -> CommandResult<UserProfile> {
    let wnacg_client = app.get_wnacg_client();

    let user_profile = wnacg_client
        .get_user_profile()
        .await
        .map_err(|err| CommandError::from("获取用户信息失败", err))?;
    tracing::debug!("获取用户信息成功");
    Ok(user_profile)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn search_by_keyword(
    app: AppHandle,
    keyword: String,
    page_num: i64,
) -> CommandResult<SearchResult> {
    let wnacg_client = app.get_wnacg_client();

    let search_result = wnacg_client
        .search_by_keyword(&keyword, page_num)
        .await
        .map_err(|err| CommandError::from("关键词搜索失败", err))?;
    tracing::debug!("关键词搜索成功");
    Ok(search_result)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn search_by_tag(
    app: AppHandle,
    tag_name: String,
    page_num: i64,
) -> CommandResult<SearchResult> {
    let wnacg_client = app.get_wnacg_client();

    let search_result = wnacg_client
        .search_by_tag(&tag_name, page_num)
        .await
        .map_err(|err| CommandError::from("按标签搜索失败", err))?;
    tracing::debug!("标签搜索成功");
    Ok(search_result)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_comic(app: AppHandle, id: i64) -> CommandResult<Comic> {
    let wnacg_client = app.get_wnacg_client();

    let comic = wnacg_client
        .get_comic(id)
        .await
        .map_err(|err| CommandError::from("获取漫画失败", err))?;
    tracing::debug!("获取漫画成功");
    Ok(comic)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_shelf(
    app: AppHandle,
    shelf_id: i64,
    page_num: i64,
) -> CommandResult<GetShelfResult> {
    let wnacg_client = app.get_wnacg_client();

    let get_shelf_result = wnacg_client
        .get_shelf(shelf_id, page_num)
        .await
        .map_err(|err| CommandError::from("获取书架失败", err))?;
    tracing::debug!("获取书架成功");
    Ok(get_shelf_result)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn create_download_task(app: AppHandle, comic: Comic) {
    let download_manager = app.get_download_manager();

    download_manager.create_download_task(comic);
    tracing::debug!("下载任务创建成功");
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn pause_download_task(app: AppHandle, comic_id: i64) -> CommandResult<()> {
    let download_manager = app.get_download_manager();

    download_manager
        .pause_download_task(comic_id)
        .map_err(|err| CommandError::from(&format!("暂停漫画ID为`{comic_id}`的下载任务"), err))?;
    tracing::debug!("暂停漫画ID为`{comic_id}`的下载任务成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn resume_download_task(app: AppHandle, comic_id: i64) -> CommandResult<()> {
    let download_manager = app.get_download_manager();

    download_manager
        .resume_download_task(comic_id)
        .map_err(|err| CommandError::from(&format!("恢复漫画ID为`{comic_id}`的下载任务"), err))?;
    tracing::debug!("恢复漫画ID为`{comic_id}`的下载任务成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn cancel_download_task(app: AppHandle, comic_id: i64) -> CommandResult<()> {
    let download_manager = app.get_download_manager();

    download_manager
        .cancel_download_task(comic_id)
        .map_err(|err| CommandError::from(&format!("取消漫画ID为`{comic_id}`的下载任务"), err))?;
    tracing::debug!("取消漫画ID为`{comic_id}`的下载任务成功");
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_downloaded_comics(app: AppHandle) -> CommandResult<Vec<Comic>> {
    let config = app.get_config();

    let download_dir = config.read().download_dir.clone();
    // 遍历下载目录，获取所有元数据文件的路径和修改时间
    let mut metadata_path_with_modify_time = std::fs::read_dir(&download_dir)
        .map_err(|err| {
            let err_title = format!(
                "获取已下载的漫画失败，读取下载目录`{}`失败",
                download_dir.display()
            );
            CommandError::from(&err_title, err)
        })?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            if entry.file_name().to_string_lossy().starts_with(".下载中-") {
                return None;
            }
            let metadata_path = entry.path().join("元数据.json");
            if !metadata_path.exists() {
                return None;
            }
            let modify_time = metadata_path.metadata().ok()?.modified().ok()?;
            Some((metadata_path, modify_time))
        })
        .collect::<Vec<_>>();
    // 按照文件修改时间排序，最新的排在最前面
    metadata_path_with_modify_time.sort_by(|(_, a), (_, b)| b.cmp(a));
    // 从元数据文件中读取Comic
    let downloaded_comics = metadata_path_with_modify_time
        .iter()
        .filter_map(
            |(metadata_path, _)| match Comic::from_metadata(&app, metadata_path) {
                Ok(comic) => Some(comic),
                Err(err) => {
                    let err_title = format!("读取元数据文件`{}`失败", metadata_path.display());
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    tracing::debug!("获取已下载的漫画成功");
    Ok(downloaded_comics)
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn export_pdf(app: AppHandle, comic: Comic) -> CommandResult<()> {
    let title = comic.title.clone();
    export::pdf(&app, &comic)
        .map_err(|err| CommandError::from(&format!("漫画`{title}`导出pdf失败"), err))?;
    tracing::debug!("漫画`{title}`导出pdf成功");
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn export_cbz(app: AppHandle, comic: Comic) -> CommandResult<()> {
    let title = comic.title.clone();
    export::cbz(&app, comic)
        .map_err(|err| CommandError::from(&format!("漫画`{title}`导出cbz失败"), err))?;
    tracing::debug!("漫画`{title}`导出cbz成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_logs_dir_size(app: AppHandle) -> CommandResult<u64> {
    let logs_dir = logger::logs_dir(&app)
        .context("获取日志目录失败")
        .map_err(|err| CommandError::from("获取日志目录大小失败", err))?;
    let logs_dir_size = std::fs::read_dir(&logs_dir)
        .context(format!("读取日志目录`{}`失败", logs_dir.display()))
        .map_err(|err| CommandError::from("获取日志目录大小失败", err))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum::<u64>();
    tracing::debug!("获取日志目录大小成功");
    Ok(logs_dir_size)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn show_path_in_file_manager(app: AppHandle, path: &str) -> CommandResult<()> {
    app.opener()
        .reveal_item_in_dir(path)
        .context(format!("在文件管理器中打开`{path}`失败"))
        .map_err(|err| CommandError::from("在文件管理器中打开失败", err))?;
    tracing::debug!("在文件管理器中打开成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub async fn get_cover_data(app: AppHandle, cover_url: String) -> CommandResult<Vec<u8>> {
    let wnacg_client = app.get_wnacg_client();

    let cover_data = wnacg_client
        .get_cover_data(&cover_url)
        .await
        .map_err(|err| CommandError::from("获取封面失败", err))?;
    Ok(cover_data.to_vec())
}

#[allow(clippy::cast_possible_wrap)]
#[tauri::command(async)]
#[specta::specta]
pub async fn download_shelf(app: AppHandle, shelf_id: i64) -> CommandResult<()> {
    let config = app.get_config();
    let wnacg_client = app.get_wnacg_client().inner().clone();
    let download_manager = app.get_download_manager();

    let mut shelf_comics = Vec::new();
    let _ = DownloadShelfEvent::GettingShelfComics.emit(&app);

    // 获取书架第一页
    let first_page = wnacg_client
        .get_shelf(shelf_id, 1)
        .await
        .context("获取书架的第`1`页失败")
        .map_err(|err| CommandError::from("下载书架失败", err))?;
    // 先把书架的第一页放进去
    shelf_comics.extend(first_page.comics);
    let page_count = first_page.total_page;
    // 获取书架剩余页
    let mut join_set = JoinSet::new();
    for page in 2..=page_count {
        let pica_client = wnacg_client.clone();
        join_set.spawn(async move {
            let page = pica_client
                .get_shelf(shelf_id, page)
                .await
                .context(format!("获取书架的第`{page}`页失败"))?;
            Ok::<_, anyhow::Error>(page)
        });
    }
    // 等待所有请求完成
    while let Some(Ok(get_shelf_result)) = join_set.join_next().await {
        // 如果有请求失败，直接返回错误
        let page = get_shelf_result.map_err(|err| CommandError::from("下载书架失败", err))?;
        shelf_comics.extend(page.comics);
    }
    // 至此，书架的漫画已经全部获取完毕
    // 去掉已下载的漫画
    shelf_comics.retain(|comic| !comic.is_downloaded);
    let total = shelf_comics.len() as i64;

    let interval_ms = config.read().download_shelf_interval_ms;
    for (i, shelf_comic) in shelf_comics.into_iter().enumerate() {
        let comic_title = &shelf_comic.title;
        let comic_id = shelf_comic.id;

        let comic = match wnacg_client
            .get_comic(comic_id)
            .await
            .context(format!("获取ID为`{comic_id}`的漫画失败"))
        {
            Ok(comic) => comic,
            Err(err) => {
                let err_title = format!("下载书架过程中，获取漫画`{comic_title}`失败，已跳过");
                let err = err.context("可能是频率太高，请手动去`配置`里调整`下载书架时，每为一本漫画创建下载任务后休息`");
                tracing::error!(err_title, message = err.to_string_chain());
                sleep(Duration::from_millis(interval_ms)).await;
                continue;
            }
        };

        let current = (i + 1) as i64;
        let _ = DownloadShelfEvent::CreatingDownloadTask { current, total }.emit(&app);

        download_manager.create_download_task(comic);
        sleep(Duration::from_millis(interval_ms)).await;
    }

    let _ = DownloadShelfEvent::End.emit(&app);

    Ok(())
}
