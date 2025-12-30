use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::types::DownloadFormat;

const DEFAULT_API_DOMAIN: &str = "www.wn06.ru";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub cookie: String,
    pub download_dir: PathBuf,
    pub export_dir: PathBuf,
    pub enable_file_logger: bool,
    pub download_format: DownloadFormat,
    pub proxy_mode: ProxyMode,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub comic_concurrency: usize,
    pub comic_download_interval_sec: u64,
    pub img_concurrency: usize,
    pub img_download_interval_sec: u64,
    pub download_shelf_interval_ms: u64,
    pub batch_download_interval_ms: u64,
    pub use_original_filename: bool,
    pub api_domain_mode: ApiDomainMode,
    pub custom_api_domain: String,
}

impl Config {
    pub fn new(app: &AppHandle) -> anyhow::Result<Config> {
        let app_data_dir = app.path().app_data_dir()?;
        let config_path = app_data_dir.join("config.json");

        let config = if config_path.exists() {
            let config_string = std::fs::read_to_string(config_path)?;
            match serde_json::from_str(&config_string) {
                // 如果能够直接解析为Config，则直接返回
                Ok(config) => config,
                // 否则，将默认配置与文件中已有的配置合并
                // 以免新版本添加了新的配置项，用户升级到新版本后，所有配置项都被重置
                Err(_) => Config::merge_config(&config_string, &app_data_dir),
            }
        } else {
            Config::default(&app_data_dir)
        };
        config.save(app)?;
        Ok(config)
    }

    pub fn save(&self, app: &AppHandle) -> anyhow::Result<()> {
        let app_data_dir = app.path().app_data_dir()?;
        let config_path = app_data_dir.join("config.json");
        let config_string = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, config_string)?;
        Ok(())
    }

    pub fn get_api_domain(&self) -> String {
        if self.api_domain_mode == ApiDomainMode::Custom {
            self.custom_api_domain.clone()
        } else {
            DEFAULT_API_DOMAIN.to_string()
        }
    }

    fn merge_config(config_string: &str, app_data_dir: &Path) -> Config {
        let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(config_string) else {
            return Config::default(app_data_dir);
        };
        let serde_json::Value::Object(ref mut map) = json_value else {
            return Config::default(app_data_dir);
        };
        let Ok(default_config_value) = serde_json::to_value(Config::default(app_data_dir)) else {
            return Config::default(app_data_dir);
        };
        let serde_json::Value::Object(default_map) = default_config_value else {
            return Config::default(app_data_dir);
        };
        for (key, value) in default_map {
            map.entry(key).or_insert(value);
        }
        let Ok(config) = serde_json::from_value(json_value) else {
            return Config::default(app_data_dir);
        };
        config
    }

    fn default(app_data_dir: &Path) -> Config {
        Config {
            cookie: String::new(),
            download_dir: app_data_dir.join("漫画下载"),
            export_dir: app_data_dir.join("漫画导出"),
            enable_file_logger: true,
            download_format: DownloadFormat::Jpeg,
            proxy_mode: ProxyMode::System,
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 7890,
            comic_concurrency: 2,
            comic_download_interval_sec: 0,
            img_concurrency: 10,
            img_download_interval_sec: 1,
            download_shelf_interval_ms: 100,
            batch_download_interval_ms: 100,
            use_original_filename: false,
            api_domain_mode: ApiDomainMode::Default,
            custom_api_domain: DEFAULT_API_DOMAIN.to_string(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub enum ProxyMode {
    #[default]
    System,
    NoProxy,
    Custom,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum ApiDomainMode {
    #[default]
    Default,
    Custom,
}
