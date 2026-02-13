use anyhow::anyhow;
use parking_lot::RwLock;
use scraper::error::SelectorErrorKind;
use tauri::{Manager, State};

use crate::{config::Config, download_manager::DownloadManager, wnacg_client::WnacgClient};

pub trait AnyhowErrorToStringChain {
    /// 将 `anyhow::Error` 转换为chain格式  
    /// # Example  
    /// 0: error message
    /// 1: error message
    /// 2: error message  
    fn to_string_chain(&self) -> String;
}

impl AnyhowErrorToStringChain for anyhow::Error {
    fn to_string_chain(&self) -> String {
        use std::fmt::Write;
        self.chain()
            .enumerate()
            .fold(String::new(), |mut output, (i, e)| {
                let _ = writeln!(output, "{i}: {e}");
                output
            })
    }
}

pub trait ToAnyhow<T> {
    fn to_anyhow(self) -> anyhow::Result<T>;
}

impl<T> ToAnyhow<T> for Result<T, SelectorErrorKind<'_>> {
    fn to_anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow!(e.to_string()))
    }
}

pub trait PathIsImg {
    /// 判断路径是否为图片(jpg/png/webp/gif)
    fn is_img(&self) -> bool;

    /// 判断路径是否为普通图片(jpg/png/webp)
    fn is_common_img(&self) -> bool;
}

impl PathIsImg for std::path::Path {
    fn is_img(&self) -> bool {
        self.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase)
            .is_some_and(|ext| matches!(ext.as_str(), "jpg" | "png" | "webp" | "gif"))
    }

    fn is_common_img(&self) -> bool {
        self.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase)
            .is_some_and(|ext| matches!(ext.as_str(), "jpg" | "png" | "webp"))
    }
}

pub trait AppHandleExt {
    fn get_config(&self) -> State<'_, RwLock<Config>>;
    fn get_wnacg_client(&self) -> State<'_, WnacgClient>;
    fn get_download_manager(&self) -> State<'_, DownloadManager>;
}

impl AppHandleExt for tauri::AppHandle {
    fn get_config(&self) -> State<'_, RwLock<Config>> {
        self.state::<RwLock<Config>>()
    }
    fn get_wnacg_client(&self) -> State<'_, WnacgClient> {
        self.state::<WnacgClient>()
    }
    fn get_download_manager(&self) -> State<'_, DownloadManager> {
        self.state::<DownloadManager>()
    }
}
