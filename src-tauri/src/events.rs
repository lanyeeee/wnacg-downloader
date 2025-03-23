use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::{
    download_manager::DownloadTaskState,
    types::{Comic, LogLevel},
};

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub timestamp: String,
    pub level: LogLevel,
    pub fields: HashMap<String, serde_json::Value>,
    pub target: String,
    pub filename: String,
    #[serde(rename = "line_number")]
    pub line_number: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskEvent {
    pub state: DownloadTaskState,
    pub comic: Comic,
    pub downloaded_img_count: u32,
    pub total_img_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSpeedEvent {
    pub speed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSleepingEvent {
    pub comic_id: i64,
    pub remaining_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(tag = "event", content = "data")]
pub enum ExportPdfEvent {
    #[serde(rename_all = "camelCase")]
    Start { uuid: String, title: String },

    #[serde(rename_all = "camelCase")]
    End { uuid: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(tag = "event", content = "data")]
pub enum ExportCbzEvent {
    #[serde(rename_all = "camelCase")]
    Start { uuid: String, title: String },

    #[serde(rename_all = "camelCase")]
    End { uuid: String },
}
