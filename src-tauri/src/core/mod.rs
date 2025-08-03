pub mod options;
pub mod task_handler;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::{sync::mpsc, task};
use weibosdk_rs::WeiboAPIImpl;

use crate::error::Result;
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderImpl;
use crate::message::Message;
use crate::storage::StorageImpl;
pub use options::{TaskOptions, UserPostFilter};
pub use task_handler::{TaskHandler, TaskRequest};

type TH =
    TaskHandler<WeiboAPIImpl<reqwest::Client>, Arc<StorageImpl>, ExporterImpl, MediaDownloaderImpl>;
pub struct Task {
    id: u64,
    total: u64,
    progress: u64,
    request: TaskRequest,
}

pub struct Core {
    next_task_id: u64,
    tasks: HashMap<u64, Task>,
    task_sender: mpsc::Sender<TaskRequest>,
}

impl Core {
    pub fn new(task_handler: TH, msg_receiver: mpsc::Receiver<Message>) -> Result<Self> {
        let (task_sender, task_receiver) = mpsc::channel(100);
        task::spawn(working_loop(task_handler, task_receiver, msg_receiver));
        Ok(Self {
            tasks: HashMap::new(),
            next_task_id: 0,
            task_sender,
        })
    }

    pub async fn backup_user(&self, options: TaskOptions) -> Result<()> {
        self.task_sender
            .send(TaskRequest::BackupUser(options))
            .await?;
        Ok(())
    }

    pub async fn backup_favorites(&self, options: TaskOptions) -> Result<()> {
        self.task_sender
            .send(TaskRequest::BackupFavorites(options))
            .await?;
        Ok(())
    }

    pub async fn unfavorite_posts(&self) -> Result<()> {
        self.task_sender.send(TaskRequest::UnfavoritePosts).await?;
        Ok(())
    }
}

async fn working_loop(
    task_handler: TH,
    mut task_receiver: mpsc::Receiver<TaskRequest>,
    mut msg_receiver: mpsc::Receiver<Message>,
) {
    let task_handler: &'static mut _ = Box::leak(Box::new(task_handler));
    loop {
        tokio::select! {
            Some(request) = task_receiver.recv() => {
                task::spawn(handle_task_request(task_handler, request));
            }
            Some(msg) = msg_receiver.recv() => {
                handle_task_responses(msg).await;
            }
            else => {
                break;
            }
        }
    }
}

// TODO
async fn handle_task_responses(msg: Message) {
    match msg {
        Message::TaskProgress(tp) => {}
        Message::UserMeta(um) => {}
        Message::Err(msg) => {}
    }
}

async fn handle_task_request(task_handler: &TH, request: TaskRequest) {
    if let Err(e) = _handle_task_request(task_handler, request).await {
        task_handler
            .msg_sender()
            .send(Message::Err(e))
            .await
            .unwrap()
    }
}

async fn _handle_task_request(task_handler: &TH, request: TaskRequest) -> Result<()> {
    // if let Some(request) = task_receiver.recv().await {
    match request {
        TaskRequest::BackupUser(options) => {
            task_handler.backup_user(options).await?;
        }
        TaskRequest::UnfavoritePosts => {
            task_handler.unfavorite_posts().await?;
        }
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(options).await?;
        }
    }
    Ok(())
}
