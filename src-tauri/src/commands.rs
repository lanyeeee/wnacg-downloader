use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::{
    config::Config,
    download_manager::DownloadManager,
    errors::{CommandError, CommandResult},
    export,
    extensions::AnyhowErrorToStringChain,
    logger,
    types::{Comic, GetFavoriteResult, SearchResult, UserProfile},
    wnacg_client::WnacgClient,
};

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_config(config: tauri::State<RwLock<Config>>) -> Config {
    let config = config.read().clone();
    tracing::debug!("获取配置成功");
    config
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn save_config(
    app: AppHandle,
    config_state: State<RwLock<Config>>,
    config: Config,
) -> CommandResult<()> {
    let enable_file_logger = config.enable_file_logger;
    let enable_file_logger_changed = config_state
        .read()
        .enable_file_logger
        .ne(&enable_file_logger);

    {
        // 包裹在大括号中，以便自动释放写锁
        let mut config_state = config_state.write();
        *config_state = config;
        config_state
            .save(&app)
            .map_err(|err| CommandError::from("保存配置失败", err))?;
        tracing::debug!("保存配置成功");
    }

    if enable_file_logger_changed {
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
pub async fn login(
    wnacg_client: State<'_, WnacgClient>,
    username: String,
    password: String,
) -> CommandResult<String> {
    let cookie = wnacg_client
        .login(&username, &password)
        .await
        .map_err(|err| CommandError::from("登录失败", err))?;
    tracing::debug!("登录成功");
    Ok(cookie)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_user_profile(wnacg_client: State<'_, WnacgClient>) -> CommandResult<UserProfile> {
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
    wnacg_client: State<'_, WnacgClient>,
    keyword: String,
    page_num: i64,
) -> CommandResult<SearchResult> {
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
    wnacg_client: State<'_, WnacgClient>,
    tag_name: String,
    page_num: i64,
) -> CommandResult<SearchResult> {
    let search_result = wnacg_client
        .search_by_tag(&tag_name, page_num)
        .await
        .map_err(|err| CommandError::from("按标签搜索失败", err))?;
    tracing::debug!("标签搜索成功");
    Ok(search_result)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_comic(wnacg_client: State<'_, WnacgClient>, id: i64) -> CommandResult<Comic> {
    let comic = wnacg_client
        .get_comic(id)
        .await
        .map_err(|err| CommandError::from("获取漫画失败", err))?;
    tracing::debug!("获取漫画成功");
    Ok(comic)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_favorite(
    wnacg_client: State<'_, WnacgClient>,
    shelf_id: i64,
    page_num: i64,
) -> CommandResult<GetFavoriteResult> {
    let get_favorite_result = wnacg_client
        .get_favorite(shelf_id, page_num)
        .await
        .map_err(|err| CommandError::from("获取收藏的漫画失败", err))?;
    tracing::debug!("获取收藏夹成功");
    Ok(get_favorite_result)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn create_download_task(download_manager: State<DownloadManager>, comic: Comic) {
    download_manager.create_download_task(comic);
    tracing::debug!("下载任务创建成功");
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn pause_download_task(
    download_manager: State<DownloadManager>,
    comic_id: i64,
) -> CommandResult<()> {
    download_manager
        .pause_download_task(comic_id)
        .map_err(|err| CommandError::from(&format!("暂停漫画ID为`{comic_id}`的下载任务"), err))?;
    tracing::debug!("暂停漫画ID为`{comic_id}`的下载任务成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn resume_download_task(
    download_manager: State<DownloadManager>,
    comic_id: i64,
) -> CommandResult<()> {
    download_manager
        .resume_download_task(comic_id)
        .map_err(|err| CommandError::from(&format!("恢复漫画ID为`{comic_id}`的下载任务"), err))?;
    tracing::debug!("恢复漫画ID为`{comic_id}`的下载任务成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn cancel_download_task(
    download_manager: State<DownloadManager>,
    comic_id: i64,
) -> CommandResult<()> {
    download_manager
        .cancel_download_task(comic_id)
        .map_err(|err| CommandError::from(&format!("取消漫画ID为`{comic_id}`的下载任务"), err))?;
    tracing::debug!("取消漫画ID为`{comic_id}`的下载任务成功");
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_downloaded_comics(
    app: AppHandle,
    config: State<RwLock<Config>>,
) -> CommandResult<Vec<Comic>> {
    let download_dir = config.read().download_dir.clone();
    // 遍历下载目录，获取所有元数据文件的路径和修改时间
    let mut metadata_path_with_modify_time = std::fs::read_dir(&download_dir)
        .map_err(|err| {
            let err_title = format!("获取已下载的漫画失败，读取下载目录 {download_dir:?} 失败");
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
        .filter_map(|(metadata_path, _)| {
            match Comic::from_metadata(&app, metadata_path).map_err(anyhow::Error::from) {
                Ok(comic) => Some(comic),
                Err(err) => {
                    let err_title = format!("读取元数据文件`{metadata_path:?}`失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    None
                }
            }
        })
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
