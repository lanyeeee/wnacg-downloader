use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::{
    config::Config,
    errors::{CommandError, CommandResult},
    logger,
    types::{SearchResult, UserProfile},
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
