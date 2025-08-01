pub mod options;
pub mod task_handler;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc::{self, error::TryRecvError};
use weibosdk_rs::{WeiboAPIImpl, client::new_client_with_headers};

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
    msg_receiver: mpsc::Receiver<Message>,
    next_task_id: u64,
    tasks: HashMap<u64, Task>,
    task_handler: TH,
    http_client: reqwest::Client,
}

impl Core {
    pub fn new() -> Result<Self> {
        let (msg_sender, msg_receiver) = mpsc::channel(100);
        let storage = StorageImpl::new(msg_sender.clone()).unwrap();
        let storage = Arc::new(storage);
        let exporter = ExporterImpl::new(msg_sender.clone());
        let http_client = new_client_with_headers().unwrap();
        let downloader = MediaDownloaderImpl::new(http_client.clone(), msg_sender.clone());
        let api_client = WeiboAPIImpl::new(http_client.clone());
        let task_handler =
            TaskHandler::new(api_client, storage, exporter, downloader, msg_sender).unwrap();
        Ok(Self {
            tasks: HashMap::new(),
            next_task_id: 0,
            msg_receiver,
            task_handler,
            http_client,
        })
    }

    pub fn task_handler(&self) -> &TH {
        &self.task_handler
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
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
