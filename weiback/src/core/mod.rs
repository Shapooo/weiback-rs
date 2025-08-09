pub mod task;
pub mod task_handler;
pub mod task_manager;

use log::{debug, error, info};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use tokio::task::spawn;
use weibosdk_rs::WeiboAPIImpl;

use crate::error::Result;
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderImpl;
use crate::message::{ErrMsg, ErrType, Message};
use crate::storage::StorageImpl;
pub use task::{BFOptions, BUOptions, Task, TaskRequest, UserPostFilter};
pub use task_handler::TaskHandler;
use task_manager::TaskManger;

type TH =
    TaskHandler<WeiboAPIImpl<reqwest::Client>, Arc<StorageImpl>, ExporterImpl, MediaDownloaderImpl>;
pub struct Core {
    next_task_id: AtomicU64,
    task_handler: &'static TH,
    task_manager: TaskManger,
}

impl Core {
    pub fn new(task_handler: TH) -> Result<Self> {
        let task_handler: &'static mut _ = Box::leak(Box::new(task_handler));
        Ok(Self {
            next_task_id: AtomicU64::new(1),
            task_handler,
            task_manager: TaskManger::new(),
        })
    }

    pub fn task_manager(&self) -> &TaskManger {
        &self.task_manager
    }

    pub async fn backup_user(&self, request: TaskRequest) -> Result<()> {
        let id = self.record_task(request.clone()).await?;
        spawn(handle_task_request(self.task_handler, id, request));
        Ok(())
    }

    pub async fn backup_favorites(&self, request: TaskRequest) -> Result<()> {
        let id = self.record_task(request.clone()).await?;
        spawn(handle_task_request(self.task_handler, id, request));
        Ok(())
    }

    pub async fn unfavorite_posts(&self) -> Result<()> {
        let id = self.record_task(TaskRequest::UnfavoritePosts).await?;
        spawn(handle_task_request(
            self.task_handler,
            id,
            TaskRequest::UnfavoritePosts,
        ));
        Ok(())
    }

    async fn record_task(&self, request: TaskRequest) -> Result<u64> {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        info!("Recording new task with id: {id}, request: {request:?}");
        let total = request.total() as u64;
        let task = Task {
            id,
            total,
            progress: 0,
            request,
        };
        self.task_manager.new_task(id, task)?;
        debug!("Task {id} recorded successfully");
        Ok(id)
    }
}

async fn handle_task_request(task_handler: &TH, task_id: u64, request: TaskRequest) {
    info!("Handling task request for task_id: {task_id}");
    debug!("Task request details: {request:?}");
    let res = match request {
        TaskRequest::BackupUser(options) => task_handler.backup_user(task_id, options).await,
        TaskRequest::UnfavoritePosts => task_handler.unfavorite_posts(task_id).await,
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(task_id, options).await
        }
    };
    if let Err(err) = res {
        error!("Task {task_id} failed: {err}");
        task_handler
            .msg_sender()
            .send(Message::Err(ErrMsg {
                r#type: ErrType::LongTaskFail { task_id },
                task_id,
                err: err.to_string(),
            }))
            .await
            .unwrap_or_else(|e| error!("Failed to send error message for task {task_id}: {e}"));
    } else {
        info!("Task {task_id} completed successfully");
    }
}
