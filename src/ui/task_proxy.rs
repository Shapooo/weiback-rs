use std::ops::RangeInclusive;

use log::{debug, error, info};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Sender};

use crate::app::service::task_handler::TaskHandler;
use crate::app::{Exporter, Network, Service, Storage, Task, TaskResponse};

pub struct TaskProxy {
    rt: Runtime,
    tx: Sender<Task>,
}

impl TaskProxy {
    pub fn new<N: Network, S: Storage, E: Exporter>(
        network: N,
        storage: S,
        exporter: E,
        task_status_sender: Sender<TaskResponse>,
    ) -> Self {
        debug!("new a executor");
        let (tx, mut rx) = channel(1);
        std::thread::spawn(move || {
            debug!("entered a new worker thread");
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            debug!("new a async runtime succeed");
            let mut th = TaskHandler::new(network, storage, exporter, task_status_sender).unwrap();
            rt.block_on(async move {
                th.init().await;
                debug!("task handler init succeed");
                while let Some(msg) = rx.recv().await {
                    debug!("worker receive msg {:?}", msg);
                    match msg {
                        Task::BackupFavorites(range, with_pic, image_definition) => {
                            th.backup_favorites(range, with_pic, image_definition).await
                        }
                        Task::ExportFromLocal(range, rev, image_definition) => {
                            th.export_from_local(range, rev, image_definition).await
                        }
                        Task::BackupUser(uid, with_pic, image_definition) => {
                            if uid == 0 {
                                th.backup_self(with_pic, image_definition).await
                            } else {
                                th.backup_user(uid, with_pic, image_definition).await
                            }
                        }
                        Task::UnfavoritePosts => th.unfavorite_posts().await,
                        Task::FetchUserMeta(id) => th.get_user_meta(id).await,
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

    fn send_task(&self, task: Task) {
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

impl TaskProxy {
    fn unfavorite_posts(&self) {
        debug!("send task: unfavorite posts");
        self.send_task(Task::UnfavoritePosts)
    }

    fn backup_fav(&self, range: RangeInclusive<u32>, with_pic: bool, image_definition: u8) {
        debug!("send task: download meta");
        self.send_task(Task::BackupFavorites(range, with_pic, image_definition))
    }

    fn backup_user(&self, uid: i64, with_pic: bool, image_definition: u8) {
        debug!("send task: backup user");
        self.send_task(Task::BackupUser(uid, with_pic, image_definition))
    }

    fn get_user_meta(&self, id: i64) {
        debug!("send task: get user meta");
        self.send_task(Task::FetchUserMeta(id))
    }

    fn export_from_local(&self, range: RangeInclusive<u32>, reverse: bool, image_definition: u8) {
        debug!("send task: export from local");
        self.send_task(Task::ExportFromLocal(range, reverse, image_definition))
    }
}
