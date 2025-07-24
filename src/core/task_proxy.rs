use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, error, info};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{Sender, channel};
use weibosdk_rs::{WeiboAPIImpl, client::new_client_with_headers, session::Session};

use super::{TaskHandler, TaskRequest};
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderImpl;
use crate::message::Message;
use crate::storage::StorageImpl;

pub struct Task {
    id: u64,
    total: u64,
    progress: u64,
    request: TaskRequest,
}

pub struct TaskProxy {
    rt: Runtime,
    tx: Sender<TaskRequest>,
    next_task_id: u64,
    tasks: HashMap<u64, Task>,
}

impl TaskProxy {
    pub fn new(msg_sender: Sender<Message>) -> Self {
        debug!("new a executor");
        let (tx, mut rx) = channel(1);
        std::thread::spawn(move || {
            debug!("entered a new worker thread");
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            debug!("new a async runtime succeed");

            rt.block_on(async move {
                let storage = StorageImpl::new(msg_sender.clone()).await.unwrap();
                let storage = Arc::new(storage);
                let exporter = ExporterImpl::new(msg_sender.clone());
                let session = Session::load("").ok(); // TODO
                let client = new_client_with_headers().unwrap();
                let api_client = session.map(|s| WeiboAPIImpl::new(client.clone(), s));
                let downloader = MediaDownloaderImpl::new(client, msg_sender.clone());
                let th = TaskHandler::new(api_client, storage, exporter, downloader, msg_sender)
                    .unwrap();

                debug!("task handler init succeed");
                while let Some(msg) = rx.recv().await {
                    debug!("worker receive msg {:?}", msg);
                    match msg {
                        TaskRequest::BackupFavorites(options) => {
                            th.backup_favorites(options).await.unwrap()
                        }
                        TaskRequest::ExportFromLocal(options) => {
                            th.export_from_local(options).await.unwrap()
                        }
                        TaskRequest::BackupUser(options) => {
                            // if uid == 0 {
                            th.backup_self(options).await.unwrap()
                            // } else {
                            //     th.backup_user(options).await
                            // }
                        }
                        TaskRequest::UnfavoritePosts => th.unfavorite_posts().await.unwrap(),
                        TaskRequest::FetchUserMeta(id) => {} // th.get_user_meta(id).await,
                    }
                }
                Ok::<(), anyhow::Error>(())
            })
            .unwrap();
        });
        Self {
            rt: Runtime::new().unwrap(),
            tx,
            next_task_id: 0,
            tasks: Default::default(),
        }
    }

    pub fn send_task(&self, task: TaskRequest) {
        match self.rt.block_on(self.tx.send(task)) {
            Ok(()) => {
                info!("task send succ")
            }
            Err(e) => {
                error!("{:?}", e);
                panic!("{:?}", e)
            }
        }
    }
}
