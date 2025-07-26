pub mod options;
pub mod task_handler;

use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, info};
use tokio::sync::mpsc::{self, error::TryRecvError};
use weibosdk_rs::{WeiboAPIImpl, client::new_client_with_headers, session::Session};

use crate::error::Result;
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderImpl;
use crate::message::{Message, UiAction};
use crate::storage::StorageImpl;
pub use options::{TaskOptions, UserPostFilter};
pub use task_handler::{TaskHandler, TaskRequest};

pub struct Task {
    id: u64,
    total: u64,
    progress: u64,
    request: TaskRequest,
}

pub struct Core {
    msg_receiver: mpsc::Receiver<Message>,
    next_task_id: u64,
    tasks: HashMap<u64, Task>,
    task_handler: TaskHandler<
        WeiboAPIImpl<reqwest::Client>,
        Arc<StorageImpl>,
        ExporterImpl,
        MediaDownloaderImpl,
    >,
}

impl Core {
    pub async fn new() -> Result<Self> {
        let (msg_sender, msg_receiver) = mpsc::channel(100);

        let storage = StorageImpl::new(msg_sender.clone()).await.unwrap();
        let storage = Arc::new(storage);
        let exporter = ExporterImpl::new(msg_sender.clone());
        let session = Session::load("").ok(); // TODO
        let client = new_client_with_headers().unwrap();
        let api_client = session.map(|s| WeiboAPIImpl::new(client.clone(), s));
        let downloader = MediaDownloaderImpl::new(client, msg_sender.clone());
        let task_handler =
            TaskHandler::new(api_client, storage, exporter, downloader, msg_sender).unwrap();

        Ok(Self {
            tasks: HashMap::new(),
            next_task_id: 0,
            msg_receiver,
            task_handler,
        })
    }

    fn handle_task_responses(&mut self) {
        let task_status: Option<Message> = match self.msg_receiver.try_recv() {
            Ok(status) => Some(status),
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{}", e),
        };

        if let Some(task_status) = task_status {
            match task_status {
                Message::TaskProgress(tp) => {}
                Message::UserMeta(um) => {}
                Message::Err(msg) => {}
            }
        }
    }

    async fn unfavorite_posts(&self) -> Result<()> {
        self.task_handler.unfavorite_posts().await
    }
}
