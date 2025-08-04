pub mod task;
pub mod task_handler;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use tokio::{sync::mpsc, task::spawn};
use weibosdk_rs::WeiboAPIImpl;

use crate::error::{Error, Result};
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderImpl;
use crate::message::Message;
use crate::storage::StorageImpl;
pub use task::{BFOptions, BUOptions, Task, TaskRequest, UserPostFilter};
pub use task_handler::TaskHandler;

type TH =
    TaskHandler<WeiboAPIImpl<reqwest::Client>, Arc<StorageImpl>, ExporterImpl, MediaDownloaderImpl>;
pub struct Core {
    next_task_id: AtomicU64,
    tasks: Arc<Mutex<HashMap<u64, Task>>>,
    task_handler: &'static TH,
}

impl Core {
    pub fn new(task_handler: TH, msg_receiver: mpsc::Receiver<Message>) -> Result<Self> {
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        spawn(msg_loop(tasks.clone(), msg_receiver));
        let task_handler: &'static mut _ = Box::leak(Box::new(task_handler));
        Ok(Self {
            tasks: tasks,
            next_task_id: AtomicU64::new(1),
            task_handler,
        })
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
        let total = request.total() as u64;
        let task = Task {
            id,
            total,
            progress: 0,
            request,
        };
        self.tasks
            .lock()
            .unwrap()
            .insert(id, task)
            .map_or(Ok(()), |_| {
                Err(Error::Other("Duplicate task id".to_string()))
            })?;
        Ok(id)
    }
}

async fn msg_loop(
    tasks: Arc<Mutex<HashMap<u64, Task>>>,
    mut msg_receiver: mpsc::Receiver<Message>,
) {
    loop {
        tokio::select! {
            Some(msg) = msg_receiver.recv() => {
                handle_task_responses(&tasks, msg).await;
            }
            else => {
                break;
            }
        }
    }
}

// TODO
async fn handle_task_responses(tasks: &Mutex<HashMap<u64, Task>>, msg: Message) {
    match msg {
        Message::TaskProgress(tp) => {}
        Message::UserMeta(um) => {}
        Message::Err(msg) => {}
    }
}

async fn handle_task_request(task_handler: &TH, id: u64, request: TaskRequest) -> Result<()> {
    // if let Some(request) = task_receiver.recv().await {
    match request {
        TaskRequest::BackupUser(options) => {
            task_handler.backup_user(id, options).await?;
        }
        TaskRequest::UnfavoritePosts => {
            task_handler.unfavorite_posts(id).await?;
        }
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(id, options).await?;
        }
    }
    Ok(())
}
