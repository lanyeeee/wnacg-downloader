use parking_lot::RwLock;
use tauri::State;

use crate::{
    config::Config, errors::{CommandError, CommandResult}, types::UserProfile, wnacg_client::WnacgClient
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
