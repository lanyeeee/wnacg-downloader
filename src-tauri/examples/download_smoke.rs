// Read-only live check: cargo run --release --example download_smoke -- <album-id> [series-id]
// Uses the application's saved session; images are validated in memory, never saved.
#![allow(dead_code, unused_imports)]

#[path = "../src/config.rs"]
mod config;
#[path = "../src/download_manager.rs"]
mod download_manager;
#[path = "../src/events.rs"]
mod events;
#[path = "../src/extensions.rs"]
mod extensions;
#[path = "../src/types/mod.rs"]
mod types;
#[path = "../src/utils.rs"]
mod utils;
#[path = "../src/wnacg_client.rs"]
mod wnacg_client;

use anyhow::{ensure, Context};
use parking_lot::RwLock;
use tauri::Manager;

fn main() -> anyhow::Result<()> {
    let ids = std::env::args()
        .skip(1)
        .map(|id| id.parse::<i64>())
        .collect::<Result<Vec<_>, _>>()?;
    ensure!(!ids.is_empty(), "provide at least one album ID");
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .setup(move |app| {
            let path = app.path().app_data_dir()?.join("config.json");
            let config: config::Config = serde_json::from_slice(&std::fs::read(path)?)?;
            if config.cookie.is_empty() {
                return Err(anyhow::anyhow!("sign in to the app before running the check").into());
            }
            app.manage(RwLock::new(config));
            let client = wnacg_client::WnacgClient::new(app.handle().clone());
            tauri::async_runtime::spawn(async move {
                let result: anyhow::Result<()> = async {
                    for id in ids {
                        let comic = client.get_comic(id).await.context("metadata/image-list request failed")?;
                        ensure!(!comic.img_list.is_empty(), "empty image list");
                        ensure!(comic.image_count as usize == comic.img_list.len(), "image count mismatch");
                        ensure!(comic.img_list.iter().all(|img| img.url.starts_with("https://")
                            && !img.url.contains("shoucang.jpg")), "invalid image URL");
                        let (bytes, format) = client.get_img_data_and_format(&comic.img_list[0].url)
                            .await.context("first image request failed")?;
                        ensure!(!bytes.is_empty(), "empty first image");
                        let cover = client.get_cover_data(&comic.cover).await.context("cover request failed")?;
                        image::guess_format(&cover).context("cover is not an image")?;
                        println!("PASS: album={id}, pages={}, first_image_bytes={}, format={format:?}, cover_bytes={}",
                            comic.image_count, bytes.len(), cover.len());
                    }
                    Ok(())
                }.await;
                if let Err(err) = result {
                    // Avoid printing page HTML, session values or signed image URLs.
                    for cause in err.chain() {
                        let text = cause.to_string();
                        let summary = text.split(['<', '\n', '\r']).next().unwrap_or("");
                        let summary = summary.split("http").next().unwrap_or("");
                        eprintln!("FAIL: {}", summary.chars().take(180).collect::<String>());
                    }
                    std::process::exit(1);
                }
                std::process::exit(0);
            });
            Ok(())
        })
        .run(context)?;
    Ok(())
}
