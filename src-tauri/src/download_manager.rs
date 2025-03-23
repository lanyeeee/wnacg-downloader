use std::{
    collections::HashMap,
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context};
use image::ImageFormat;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::{
    sync::{watch, Semaphore, SemaphorePermit},
    task::JoinSet,
};

use crate::{
    config::Config,
    events::{DownloadSpeedEvent, DownloadTaskEvent},
    extensions::AnyhowErrorToStringChain,
    types::Comic,
    wnacg_client::WnacgClient,
};

/// 用于管理下载任务
///
/// 克隆 `DownloadManager` 的开销极小，性能开销几乎可以忽略不计。
/// 可以放心地在多个线程中传递和使用它的克隆副本。
///
/// 具体来说：
/// - `app` 是 `AppHandle` 类型，根据 `Tauri` 文档，它的克隆开销是极小的。
/// - 其他字段都被 `Arc` 包裹，这些字段的克隆操作仅仅是增加引用计数。
#[derive(Clone)]
pub struct DownloadManager {
    app: AppHandle,
    comic_sem: Arc<Semaphore>,
    img_sem: Arc<Semaphore>,
    byte_per_sec: Arc<AtomicU64>,
    download_tasks: Arc<RwLock<HashMap<i64, DownloadTask>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum DownloadTaskState {
    Pending,
    Downloading,
    Paused,
    Cancelled,
    Completed,
    Failed,
}

impl DownloadManager {
    pub fn new(app: &AppHandle) -> Self {
        let (comic_concurrency, img_concurrency) = {
            let config = app.state::<RwLock<Config>>();
            let config = config.read();
            (config.comic_concurrency, config.img_concurrency)
        };

        let manager = DownloadManager {
            app: app.clone(),
            comic_sem: Arc::new(Semaphore::new(comic_concurrency)),
            img_sem: Arc::new(Semaphore::new(img_concurrency)),
            byte_per_sec: Arc::new(AtomicU64::new(0)),
            download_tasks: Arc::new(RwLock::new(HashMap::new())),
        };

        tauri::async_runtime::spawn(manager.clone().emit_download_speed_loop());

        manager
    }

    pub fn create_download_task(&self, comic: Comic) {
        use DownloadTaskState::{Downloading, Paused, Pending};
        let comic_id = comic.id;
        let mut tasks = self.download_tasks.write();
        if let Some(task) = tasks.get(&comic_id) {
            // 如果任务已经存在，且状态是`Pending`、`Downloading`或`Paused`，则不创建新任务
            let state = *task.state_sender.borrow();
            if matches!(state, Pending | Downloading | Paused) {
                return;
            }
        }
        let task = DownloadTask::new(self.app.clone(), comic);
        tauri::async_runtime::spawn(task.clone().process());
        tasks.insert(comic_id, task);
    }

    pub fn pause_download_task(&self, comic_id: i64) -> anyhow::Result<()> {
        let tasks = self.download_tasks.read();
        let Some(task) = tasks.get(&comic_id) else {
            return Err(anyhow!("未找到漫画ID为`{comic_id}`的下载任务"));
        };
        task.set_state(DownloadTaskState::Paused);
        Ok(())
    }

    pub fn resume_download_task(&self, comic_id: i64) -> anyhow::Result<()> {
        use DownloadTaskState::{Cancelled, Completed, Failed, Pending};
        let comic = {
            let tasks = self.download_tasks.read();
            let Some(task) = tasks.get(&comic_id) else {
                return Err(anyhow!("未找到漫画ID为`{comic_id}`的下载任务"));
            };
            let task_state = *task.state_sender.borrow();

            if matches!(task_state, Failed | Cancelled | Completed) {
                // 如果任务状态是`Failed`、`Cancelled`或`Completed`，则获取 comic 用于重新创建下载任务
                Some(task.comic.as_ref().clone())
            } else {
                task.set_state(Pending);
                None
            }
        };
        // 如果 comic 不为 None，则重新创建下载任务
        if let Some(comic) = comic {
            self.create_download_task(comic);
        }
        Ok(())
    }

    pub fn cancel_download_task(&self, comic_id: i64) -> anyhow::Result<()> {
        let tasks = self.download_tasks.read();
        let Some(task) = tasks.get(&comic_id) else {
            return Err(anyhow!("未找到漫画ID为`{comic_id}`的下载任务"));
        };
        task.set_state(DownloadTaskState::Cancelled);
        Ok(())
    }

    #[allow(clippy::cast_precision_loss)]
    async fn emit_download_speed_loop(self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            let byte_per_sec = self.byte_per_sec.swap(0, Ordering::Relaxed);
            let mega_byte_per_sec = byte_per_sec as f64 / 1024.0 / 1024.0;
            let speed = format!("{mega_byte_per_sec:.2} MB/s");
            // 发送总进度条下载速度事件
            let _ = DownloadSpeedEvent { speed }.emit(&self.app);
        }
    }
}

#[derive(Clone)]
struct DownloadTask {
    app: AppHandle,
    download_manager: DownloadManager,
    comic: Arc<Comic>,
    state_sender: watch::Sender<DownloadTaskState>,
    downloaded_img_count: Arc<AtomicU32>,
    total_img_count: Arc<AtomicU32>,
}

impl DownloadTask {
    pub fn new(app: AppHandle, comic: Comic) -> Self {
        let download_manager = app.state::<DownloadManager>().inner().clone();
        let (state_sender, _) = watch::channel(DownloadTaskState::Pending);
        Self {
            app,
            download_manager,
            comic: Arc::new(comic),
            state_sender,
            downloaded_img_count: Arc::new(AtomicU32::new(0)),
            total_img_count: Arc::new(AtomicU32::new(0)),
        }
    }

    async fn process(self) {
        let download_comic_task = self.download_comic();
        tokio::pin!(download_comic_task);

        let mut state_receiver = self.state_sender.subscribe();
        state_receiver.mark_changed();
        let mut permit = None;
        loop {
            let state_is_downloading = *state_receiver.borrow() == DownloadTaskState::Downloading;
            let state_is_pending = *state_receiver.borrow() == DownloadTaskState::Pending;
            tokio::select! {
                () = &mut download_comic_task, if state_is_downloading && permit.is_some() => break,
                control_flow = self.acquire_comic_permit(&mut permit), if state_is_pending => {
                    match control_flow {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                },
                _ = state_receiver.changed() => {
                    match self.handle_state_change(&mut permit, &mut state_receiver) {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                }
            }
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn download_comic(&self) {
        let comic_id = self.comic.id;
        let comic_title = &self.comic.title;
        // 获取此漫画每张图片的下载链接
        let img_urls = self
            .comic
            .img_list
            .iter()
            .map(|img| &img.url)
            .filter(|url| !url.ends_with("shoucang.jpg")) // 过滤掉最后一张图片
            .map(|url| format!("https:{url}"))
            .collect::<Vec<_>>();
        // 总共需要下载的图片数量
        self.total_img_count
            .store(img_urls.len() as u32, Ordering::Relaxed);

        // 创建临时下载目录
        let Some(temp_download_dir) = self.create_temp_download_dir() else {
            return;
        };
        // 清理临时下载目录中与`config.download_format`对不上的文件
        self.clean_temp_download_dir(&temp_download_dir);

        let mut join_set = JoinSet::new();
        // 开始下载之前，先保存元数据
        if let Err(err) = self.save_metadata(&temp_download_dir) {
            let err_title = format!("`{comic_title}`保存元数据失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
            return;
        }
        // 逐一创建下载任务
        for (i, url) in img_urls.into_iter().enumerate() {
            let url = url.clone();
            let temp_download_dir = temp_download_dir.clone();
            let download_img_task = DownloadImgTask::new(self, url, temp_download_dir, i);
            // 创建下载任务
            join_set.spawn(download_img_task.process());
        }
        // 等待所有下载任务完成
        join_set.join_all().await;
        tracing::trace!(comic_id, comic_title, "所有图片下载任务完成");
        // 检查此漫画的图片是否全部下载成功
        let downloaded_img_count = self.downloaded_img_count.load(Ordering::Relaxed);
        let total_img_count = self.total_img_count.load(Ordering::Relaxed);
        // 此漫画的图片未全部下载成功
        if downloaded_img_count != total_img_count {
            let err_title = format!("`{comic_title}`下载不完整");
            let err_msg =
                format!("总共有`{total_img_count}`张图片，但只下载了`{downloaded_img_count}`张");
            tracing::error!(err_title, message = err_msg);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_event();

            return;
        }
        // 此漫画的图片全部下载成功
        if let Err(err) = self.rename_temp_download_dir(&temp_download_dir) {
            let err_title = format!("`{comic_title}`重命名临时下载目录失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_event();

            return;
        };
        tracing::trace!(
            comic_id,
            comic_title,
            "重命名临时下载目录`{temp_download_dir:?}`成功"
        );
        tracing::info!(comic_id, comic_title, "漫画下载成功");
        // 发送下载结束事件
        self.set_state(DownloadTaskState::Completed);
        self.emit_download_task_event();
    }

    fn create_temp_download_dir(&self) -> Option<PathBuf> {
        let comic_id = self.comic.id;
        let comic_title = &self.comic.title;

        let temp_download_dir = self
            .app
            .state::<RwLock<Config>>()
            .read()
            .download_dir
            .join(format!(".下载中-{comic_title}")); // 以 `.下载中-` 开头，表示是临时目录

        if let Err(err) = std::fs::create_dir_all(&temp_download_dir).map_err(anyhow::Error::from) {
            // 如果创建目录失败，则发送下载漫画结束事件，并返回
            let err_title = format!("`{comic_title}`创建目录`{temp_download_dir:?}`失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_event();

            return None;
        };

        tracing::trace!(
            comic_id,
            comic_title,
            "创建临时下载目录`{temp_download_dir:?}`成功"
        );

        Some(temp_download_dir)
    }

    /// 删除临时下载目录中与`config.download_format`对不上的文件
    fn clean_temp_download_dir(&self, temp_download_dir: &Path) {
        let comic_id = self.comic.id;
        let comic_title = &self.comic.title;

        let entries = match std::fs::read_dir(temp_download_dir).map_err(anyhow::Error::from) {
            Ok(entries) => entries,
            Err(err) => {
                let err_title =
                    format!("`{comic_title}`读取临时下载目录`{temp_download_dir:?}`失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                return;
            }
        };

        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        let extension = download_format.extension();
        for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            // path有扩展名，且能转换为utf8，并与`config.download_format`一致，才保留
            let should_keep = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| Some(ext) == extension);
            if should_keep {
                continue;
            }
            // 否则删除文件
            if let Err(err) = std::fs::remove_file(&path).map_err(anyhow::Error::from) {
                let err_title = format!("`{comic_title}`删除临时下载目录的`{path:?}`失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
            }
        }

        tracing::trace!(
            comic_id,
            comic_title,
            "清理临时下载目录`{temp_download_dir:?}`成功"
        );
    }

    async fn acquire_comic_permit<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
    ) -> ControlFlow<()> {
        let comic_id = self.comic.id;
        let comic_title = &self.comic.title;

        tracing::debug!(comic_id, comic_title, "漫画开始排队");

        self.emit_download_task_event();

        *permit = match permit.take() {
            // 如果有permit，则直接用
            Some(permit) => Some(permit),
            // 如果没有permit，则获取permit
            None => match self
                .download_manager
                .comic_sem
                .acquire()
                .await
                .map_err(anyhow::Error::from)
            {
                Ok(permit) => Some(permit),
                Err(err) => {
                    let err_title = format!("`{comic_title}`获取下载漫画的permit失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);

                    self.set_state(DownloadTaskState::Failed);
                    self.emit_download_task_event();

                    return ControlFlow::Break(());
                }
            },
        };
        // 如果当前任务状态不是`Pending`，则不将任务状态设置为`Downloading`
        if *self.state_sender.borrow() != DownloadTaskState::Pending {
            return ControlFlow::Continue(());
        }
        // 将任务状态设置为`Downloading`
        if let Err(err) = self
            .state_sender
            .send(DownloadTaskState::Downloading)
            .map_err(anyhow::Error::from)
        {
            let err_title = format!("`{comic_title}`发送状态`Downloading`失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    fn handle_state_change<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
        state_receiver: &mut watch::Receiver<DownloadTaskState>,
    ) -> ControlFlow<()> {
        let comic_id = self.comic.id;
        let comic_title = &self.comic.title;

        self.emit_download_task_event();
        let state = *state_receiver.borrow();
        match state {
            DownloadTaskState::Paused => {
                tracing::debug!(comic_id, comic_title, "漫画暂停中");
                if let Some(permit) = permit.take() {
                    drop(permit);
                };
                ControlFlow::Continue(())
            }
            DownloadTaskState::Cancelled => {
                tracing::debug!(comic_id, comic_title, "漫画取消下载");
                ControlFlow::Break(())
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn set_state(&self, state: DownloadTaskState) {
        let comic_title = &self.comic.title;
        if let Err(err) = self.state_sender.send(state).map_err(anyhow::Error::from) {
            let err_title = format!("`{comic_title}`发送状态`{state:?}`失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
        }
    }

    fn emit_download_task_event(&self) {
        let _ = DownloadTaskEvent {
            state: *self.state_sender.borrow(),
            comic: self.comic.as_ref().clone(),
            downloaded_img_count: self.downloaded_img_count.load(Ordering::Relaxed),
            total_img_count: self.total_img_count.load(Ordering::Relaxed),
        }
        .emit(&self.app);
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn save_metadata(&self, temp_download_dir: &Path) -> anyhow::Result<()> {
        let mut comic = self.comic.as_ref().clone();
        // 将所有comic的is_downloaded字段设置为None，这样能使is_downloaded字段在序列化时被忽略
        comic.is_downloaded = None;

        let comic_title = &comic.title;
        let comic_json = serde_json::to_string_pretty(&comic).context(format!(
            "`{comic_title}`的元数据保存失败，将Comic序列化为json失败"
        ))?;

        let metadata_path = temp_download_dir.join("元数据.json");

        std::fs::write(&metadata_path, comic_json).context(format!(
            "`{comic_title}`的元数据保存失败，写入文件`{metadata_path:?}`失败"
        ))?;

        Ok(())
    }

    fn rename_temp_download_dir(&self, temp_download_dir: &Path) -> anyhow::Result<()> {
        let Some(parent) = temp_download_dir.parent() else {
            return Err(anyhow!("无法获取`{temp_download_dir:?}`的父目录"));
        };

        let download_dir = parent.join(&self.comic.title);

        if download_dir.exists() {
            std::fs::remove_dir_all(&download_dir)
                .context(format!("删除目录`{download_dir:?}`失败"))?;
        }

        std::fs::rename(temp_download_dir, &download_dir).context(format!(
            "将`{temp_download_dir:?}`重命名为`{download_dir:?}`失败"
        ))?;

        Ok(())
    }
}

#[derive(Clone)]
struct DownloadImgTask {
    app: AppHandle,
    download_manager: DownloadManager,
    download_task: DownloadTask,
    url: String,
    temp_download_dir: PathBuf,
    index: usize,
}

impl DownloadImgTask {
    pub fn new(
        download_task: &DownloadTask,
        url: String,
        temp_download_dir: PathBuf,
        index: usize,
    ) -> Self {
        Self {
            app: download_task.app.clone(),
            download_manager: download_task.download_manager.clone(),
            download_task: download_task.clone(),
            url,
            temp_download_dir,
            index,
        }
    }

    async fn process(self) {
        let download_img_task = self.download_img();
        tokio::pin!(download_img_task);

        let mut state_receiver = self.download_task.state_sender.subscribe();
        state_receiver.mark_changed();
        let mut permit = None;

        loop {
            let state_is_downloading = *state_receiver.borrow() == DownloadTaskState::Downloading;
            tokio::select! {
                () = &mut download_img_task, if state_is_downloading && permit.is_some() => break,
                control_flow = self.acquire_img_permit(&mut permit), if state_is_downloading && permit.is_none() => {
                    match control_flow {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                },
                _ = state_receiver.changed() => {
                    match self.handle_state_change(&mut permit, &mut state_receiver) {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                }
            }
        }
    }

    async fn download_img(&self) {
        let url = &self.url;
        let comic_id = self.download_task.comic.id;
        let comic_title = &self.download_task.comic.title;

        tracing::trace!(comic_id, comic_title, url, "开始下载图片");

        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        if let Some(extension) = download_format.extension() {
            // 如果图片已存在，则跳过下载
            let save_path = self
                .temp_download_dir
                .join(format!("{:04}.{extension}", self.index + 1));
            if save_path.exists() {
                tracing::trace!(comic_id, comic_title, url, "图片已存在，跳过下载");
                self.download_task
                    .downloaded_img_count
                    .fetch_add(1, Ordering::Relaxed);
                self.download_task.emit_download_task_event();
                return;
            }
        }
        // 下载图片
        let (img_data, img_format) = match self.wnacg_client().get_img_data_and_format(url).await {
            Ok(data_and_format) => data_and_format,
            Err(err) => {
                let err_title = format!("下载图片`{url}`失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                return;
            }
        };

        tracing::trace!(comic_id, comic_title, url, "图片成功下载到内存");

        // 获取图片格式的扩展名
        let extension = match img_format {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::WebP => "webp",
            _ => {
                let err_title = format!("保存图片`{url}`失败");
                let err_msg = format!("{img_format:?}格式不支持");
                tracing::error!(err_title, message = err_msg);
                return;
            }
        };

        let save_path = self
            .temp_download_dir
            .join(format!("{:04}.{extension}", self.index + 1));
        // 保存图片
        if let Err(err) = std::fs::write(&save_path, &img_data).map_err(anyhow::Error::from) {
            let err_title = format!("保存图片`{save_path:?}`失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
            return;
        }
        tracing::trace!(comic_id, url, comic_title, "图片成功保存到`{save_path:?}`");
        // 记录下载字节数
        self.download_manager
            .byte_per_sec
            .fetch_add(img_data.len() as u64, Ordering::Relaxed);
        tracing::trace!(comic_id, url, comic_title, "图片下载成功");

        self.download_task
            .downloaded_img_count
            .fetch_add(1, Ordering::Relaxed);
        self.download_task.emit_download_task_event();
    }

    async fn acquire_img_permit<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
    ) -> ControlFlow<()> {
        let url = &self.url;
        let comic_id = self.download_task.comic.id;
        let comic_title = &self.download_task.comic.title;

        tracing::trace!(comic_id, comic_title, url, "图片开始排队");

        *permit = match permit.take() {
            // 如果有permit，则直接用
            Some(permit) => Some(permit),
            // 如果没有permit，则获取permit
            None => match self
                .download_manager
                .img_sem
                .acquire()
                .await
                .map_err(anyhow::Error::from)
            {
                Ok(permit) => Some(permit),
                Err(err) => {
                    let err_title = format!("`{comic_title}`获取下载图片的permit失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    return ControlFlow::Break(());
                }
            },
        };
        ControlFlow::Continue(())
    }

    fn handle_state_change<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
        state_receiver: &mut watch::Receiver<DownloadTaskState>,
    ) -> ControlFlow<()> {
        let url = &self.url;
        let comic_id = self.download_task.comic.id;
        let comic_title = &self.download_task.comic.title;

        let state = *state_receiver.borrow();
        match state {
            DownloadTaskState::Paused => {
                tracing::trace!(comic_id, comic_title, url, "图片暂停下载");
                if let Some(permit) = permit.take() {
                    drop(permit);
                };
                ControlFlow::Continue(())
            }
            DownloadTaskState::Cancelled => {
                tracing::trace!(comic_id, comic_title, url, "图片取消下载");
                ControlFlow::Break(())
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn wnacg_client(&self) -> WnacgClient {
        self.app.state::<WnacgClient>().inner().clone()
    }
}
