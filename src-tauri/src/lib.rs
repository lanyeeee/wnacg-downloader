mod commands;
mod config;
mod errors;
mod events;
mod extensions;
mod logger;
mod types;
mod utils;
mod wnacg_client;

use anyhow::Context;
use config::Config;
use events::LogEvent;
use parking_lot::RwLock;
use tauri::{Manager, Wry};
use wnacg_client::WnacgClient;

use crate::commands::*;

fn generate_context() -> tauri::Context<Wry> {
    tauri::generate_context!()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<Wry>::new()
        .commands(tauri_specta::collect_commands![
            greet,
            get_config,
            save_config,
            login,
            get_user_profile,
            search_by_keyword,
            search_by_tag,
            get_comic,
            get_favorite,
        ])
        .events(tauri_specta::collect_events![LogEvent]);

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number)
                .formatter(specta_typescript::formatter::prettier)
                .header("// @ts-nocheck"), // 跳过检查
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let app_data_dir = app
                .path()
                .app_data_dir()
                .context("获取app_data_dir目录失败")?;

            std::fs::create_dir_all(&app_data_dir)
                .context(format!("创建app_data_dir目录`{app_data_dir:?}`失败"))?;

            let config = RwLock::new(Config::new(app.handle())?);
            app.manage(config);

            let wnacg_client = WnacgClient::new(app.handle().clone());
            app.manage(wnacg_client);

            logger::init(app.handle())?;

            Ok(())
        })
        .run(generate_context())
        .expect("error while running tauri application");
}
