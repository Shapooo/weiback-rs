use std::ops::RangeInclusive;

use log::{debug, error, info};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{Sender, channel};
use weibosdk_rs::WeiboAPIImpl;

use crate::app::TaskHandler;
use crate::ports::{
    ExportOptions, PictureDefinition, Service, Storage, Task, TaskOptions, TaskResponse,
};

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

    fn unfavorite_posts(&self) {
        debug!("send task: unfavorite posts");
        self.send_task(Task::UnfavoritePosts)
    }

    fn backup_fav(&self, range: RangeInclusive<u32>, with_pic: bool, pic_def: PictureDefinition) {
        debug!("send task: download meta");
        let options = TaskOptions::new()
            .pic_quality(pic_def)
            .range(range)
            .with_pic(with_pic);
        self.send_task(Task::BackupFavorites(options))
    }

    fn backup_user(&self, uid: i64, with_pic: bool, pic_def: PictureDefinition) {
        debug!("send task: backup user");
        let options = TaskOptions::new()
            .with_user(uid)
            .with_pic(with_pic)
            .pic_quality(pic_def);
        self.send_task(Task::BackupUser(options))
    }

    fn get_user_meta(&self, id: i64) {
        debug!("send task: get user meta");
        let options = TaskOptions::new().with_user(id);
        self.send_task(Task::FetchUserMeta(options))
    }

    fn export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        pic_def: PictureDefinition,
    ) {
        debug!("send task: export from local");
        let options = ExportOptions::new()
            .reverse(reverse)
            .range(range)
            .pic_quality(pic_def);
        self.send_task(Task::ExportFromLocal(options))
    }
}
