use std::ops::RangeInclusive;
use std::sync::{Arc, RwLock};

use log::debug;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::message::Task;
use crate::message::TaskStatus;
use crate::task_handler::TaskHandler;

pub struct Executor {
    rt: Runtime,
    tx: mpsc::Sender<Task>,
}

impl Executor {
    pub fn new(config: Config, task_status: Arc<RwLock<TaskStatus>>) -> Self {
        debug!("new a executor");
        let (tx, mut rx) = mpsc::channel(1);
        std::thread::spawn(move || {
            debug!("entered a new worker thread");
            // let rt = Runtime::new().unwrap();
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            debug!("new a async runtime succeed");
            let mut th = TaskHandler::new(config, task_status).unwrap();
            rt.block_on(async move {
                th.init().await?;
                debug!("task handler init succeed");
                while let Some(msg) = rx.recv().await {
                    debug!("worker receive msg {:?}", msg);
                    match msg {
                        Task::DownloadMeta(range) => th.download_meta_only(range).await?,
                        Task::DownloadWithPic(range) => th.download_with_pic(range).await?,
                        Task::ExportFromNet(range) => th.export_from_net(range).await?,
                        Task::ExportFromLocal(range, rev) => {
                            th.export_from_local(range, rev).await?
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            })
        });
        Self {
            rt: Runtime::new().unwrap(),
            tx,
        }
    }

    pub fn download_meta(&self, range: RangeInclusive<u32>) {
        debug!("send task: download meta");
        self.rt
            .block_on(self.tx.send(Task::DownloadMeta(range)))
            .unwrap();
    }

    pub fn download_with_pic(&self, range: RangeInclusive<u32>) {
        debug!("send task: download with pic");
        self.rt
            .block_on(self.tx.send(Task::DownloadWithPic(range)))
            .unwrap();
    }
    pub fn export_from_net(&self, range: RangeInclusive<u32>) {
        debug!("send task: download and export");
        self.rt
            .block_on(self.tx.send(Task::ExportFromNet(range)))
            .unwrap();
    }
    pub fn export_from_local(&self, range: RangeInclusive<u32>, reverse: bool) {
        debug!("send task: export from local");
        self.rt
            .block_on(self.tx.send(Task::ExportFromLocal(range, reverse)))
            .unwrap();
    }
}
