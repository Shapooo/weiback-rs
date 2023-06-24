use std::ops::RangeInclusive;

use log::debug;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::message::Message;
use crate::task_handler::TaskHandler;

pub struct Executor {
    rt: Runtime,
    tx: mpsc::Sender<Message>,
}

impl Executor {
    pub fn new(config: Config) -> Self {
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
            let mut th = TaskHandler::new(config).unwrap();
            rt.block_on(async move {
                th.init().await?;
                debug!("task handler init succeed");
                while let Some(msg) = rx.recv().await {
                    debug!("worker receive msg {:?}", msg);
                    match msg {
                        Message::DownloadMeta(range) => th.download_meta_only(range).await?,
                        Message::DownloadWithPic(range) => th.download_with_pic(range).await?,
                        Message::ExportFromNet(range) => th.export_from_net(range).await?,
                        Message::ExportFromLocal(range, rev) => {
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
            .block_on(self.tx.send(Message::DownloadMeta(range)))
            .unwrap();
    }

    pub fn download_with_pic(&self, range: RangeInclusive<u32>) {
        debug!("send task: download with pic");
        self.rt
            .block_on(self.tx.send(Message::DownloadWithPic(range)))
            .unwrap();
    }
    pub fn export_from_net(&self, range: RangeInclusive<u32>) {
        debug!("send task: download and export");
        self.rt
            .block_on(self.tx.send(Message::ExportFromNet(range)))
            .unwrap();
    }
    pub fn export_from_local(&self, range: RangeInclusive<u32>, reverse: bool) {
        debug!("send task: export from local");
        self.rt
            .block_on(self.tx.send(Message::ExportFromLocal(range, reverse)))
            .unwrap();
    }
}
