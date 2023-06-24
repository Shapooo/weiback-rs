use std::ops::RangeInclusive;

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
        let (tx, mut rx) = mpsc::channel(1);
        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            let mut th = TaskHandler::new(config).unwrap();
            rt.block_on(async {
                th.init().await?;
                while let Some(msg) = rx.recv().await {
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
        self.rt
            .block_on(self.tx.send(Message::DownloadMeta(range)))
            .unwrap();
    }

    pub fn download_with_pic(&self, range: RangeInclusive<u32>) {
        self.rt
            .block_on(self.tx.send(Message::DownloadWithPic(range)))
            .unwrap();
    }
    pub fn export_from_net(&self, range: RangeInclusive<u32>) {
        self.rt
            .block_on(self.tx.send(Message::ExportFromNet(range)))
            .unwrap();
    }
    pub fn export_from_local(&self, range: RangeInclusive<u32>, reverse: bool) {
        self.rt
            .block_on(self.tx.send(Message::ExportFromLocal(range, reverse)))
            .unwrap();
    }
}
