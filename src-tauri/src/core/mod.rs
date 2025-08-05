pub mod task;
pub mod task_handler;

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use tauri::{AppHandle, Emitter};
use tokio::{sync::mpsc, task::spawn};
use weibosdk_rs::WeiboAPIImpl;

use crate::error::{Error, Result};
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderImpl;
use crate::message::{ErrMsg, ErrType, Message, TaskProgress};
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
    pub fn new(
        app: AppHandle,
        task_handler: TH,
        msg_receiver: mpsc::Receiver<Message>,
    ) -> Result<Self> {
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        spawn(msg_loop(app, tasks.clone(), msg_receiver));
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
    app: AppHandle,
    tasks: Arc<Mutex<HashMap<u64, Task>>>,
    mut msg_receiver: mpsc::Receiver<Message>,
) {
    loop {
        tokio::select! {
            Some(msg) = msg_receiver.recv() => {
                handle_task_responses(&app, &tasks, msg).await;
            }
            else => {
                break;
            }
        }
    }
}

async fn handle_task_responses(
    app: &AppHandle,
    tasks: &Mutex<HashMap<u64, Task>>,
    msg: Message,
) -> Result<()> {
    match msg {
        Message::TaskProgress(TaskProgress {
            r#type,
            task_id,
            total_increment,
            progress_increment,
        }) => {
            let mut tasks = tasks.lock().map_err(|err| Error::Other(err.to_string()))?;
            let mut total_new = 0;
            let mut progress_new = 0;
            tasks.entry(task_id).and_modify(
                |Task {
                     total, progress, ..
                 }| {
                    *total += total_increment;
                    *progress += progress_increment;
                    total_new = *total;
                    progress_new = *progress;
                },
            );
            app.emit(
                "task-progress",
                serde_json::json!({
                    "type":r#type,
                    "total":total_new,
                    "progress":progress_new
                }),
            )?;
        }
        Message::Err(ErrMsg {
            task_id,
            err,
            r#type,
        }) => app.emit(
            "error",
            serde_json::json!({
                "type": r#type,
                "task_id":task_id,
                "err": err.to_string(),
            }),
        )?,
    }
    Ok(())
}

async fn handle_task_request(task_handler: &TH, task_id: u64, request: TaskRequest) {
    let res = match request {
        TaskRequest::BackupUser(options) => task_handler.backup_user(task_id, options).await,
        TaskRequest::UnfavoritePosts => task_handler.unfavorite_posts(task_id).await,
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(task_id, options).await
        }
    };
    if let Err(err) = res {
        task_handler
            .msg_sender()
            .send(Message::Err(ErrMsg {
                r#type: ErrType::LongTaskFail { task_id },
                task_id,
                err: err.to_string(),
            }))
            .await
            .unwrap();
    }
}
