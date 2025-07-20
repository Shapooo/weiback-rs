use std::ops::RangeInclusive;
use std::sync::Arc;

use log::{debug, error, info};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{Sender, channel};
use weibosdk_rs::{WeiboAPIImpl, client::new_client_with_headers, session::Session};

use crate::app::{Task, TaskHandler, TaskOptions, TaskResponse};
use crate::exporter::{ExportOptions, ExporterImpl};
use crate::models::PictureDefinition;
use crate::storage::StorageImpl;

pub struct TaskProxy {
    rt: Runtime,
    tx: Sender<Task>,
}

impl TaskProxy {
    pub fn new(task_status_sender: Sender<TaskResponse>) -> Self {
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
                let storage = StorageImpl::new().await.unwrap();
                let storage = Arc::new(storage);
                let exporter = ExporterImpl();
                let session = Session::load("").ok(); // TODO
                let api_client =
                    session.map(|s| WeiboAPIImpl::new(new_client_with_headers().unwrap(), s));
                let th =
                    TaskHandler::new(api_client, storage, exporter, task_status_sender).unwrap();

                debug!("task handler init succeed");
                while let Some(msg) = rx.recv().await {
                    debug!("worker receive msg {:?}", msg);
                    match msg {
                        Task::BackupFavorites(options) => {
                            th.backup_favorites(options).await.unwrap()
                        }
                        Task::ExportFromLocal(options) => {
                            th.export_from_local(options).await.unwrap()
                        }
                        Task::BackupUser(options) => {
                            // if uid == 0 {
                            th.backup_self(options).await.unwrap()
                            // } else {
                            //     th.backup_user(options).await
                            // }
                        }
                        Task::UnfavoritePosts => th.unfavorite_posts().await.unwrap(),
                        Task::FetchUserMeta(id) => {} // th.get_user_meta(id).await,
                    }
                }
                Ok::<(), anyhow::Error>(())
            })
            .unwrap();
        });
        Self {
            rt: Runtime::new().unwrap(),
            tx,
        }
    }

    pub fn send_task(&self, task: Task) {
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
