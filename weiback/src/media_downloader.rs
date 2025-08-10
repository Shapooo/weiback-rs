#![allow(async_fn_in_trait)]
use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use log::{debug, error, info};
use reqwest::Client;
use tokio::sync::mpsc;

use crate::error::Result;
use crate::message::{ErrMsg, ErrType, Message};

pub trait MediaDownloader {
    async fn download_picture(
        &self,
        task_id: u64,
        url: String,
        callback: AsyncDownloadCallback,
    ) -> Result<()>;
}

/// The callback is for success cases and is async.
type AsyncDownloadCallback = Box<
    dyn FnOnce(Bytes) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>
        + Send
        + 'static,
>;

/// A task for the media downloader actor.
struct DownloadTask {
    task_id: u64,
    url: String,
    callback: AsyncDownloadCallback,
}

/// The worker that runs in a background task.
/// It owns the receiver, msg_sender and the HTTP client,
/// and is consumed when its `run` method is called.
#[must_use = "The worker must be spawned to process download requests"]
pub struct DownloaderWorker {
    receiver: mpsc::Receiver<DownloadTask>,
    client: Client,
    msg_sender: mpsc::Sender<Message>,
}

/// A media downloader that handles downloading pictures in a separate actor.
#[derive(Debug, Clone)]
pub struct MediaDownloaderHandle {
    sender: mpsc::Sender<DownloadTask>,
}

/// Creates a new media downloader handle and its associated worker.
///
/// # Returns
/// A tuple containing:
/// 1. `DownloaderHandle`: The handle to send download requests.
/// 2. `DownloaderWorker`: The worker that must be spawned to handle the requests.
pub fn create_downloader(
    buffer: usize,
    client: Client,
    msg_sender: mpsc::Sender<Message>,
) -> (MediaDownloaderHandle, DownloaderWorker) {
    let (sender, receiver) = mpsc::channel(buffer);
    let handle = MediaDownloaderHandle { sender };
    let worker = DownloaderWorker {
        receiver,
        client,
        msg_sender,
    };
    (handle, worker)
}

impl MediaDownloader for MediaDownloaderHandle {
    /// Queues a picture for download.
    ///
    /// This method sends a task to the background downloader actor and returns immediately.
    /// The provided async callback will be executed once the download is complete and successful.
    /// If the download fails, the task is discarded and the callback is never called.
    async fn download_picture(
        &self,
        task_id: u64,
        url: String,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
        let task = DownloadTask {
            task_id,
            url,
            callback,
        };
        Ok(self.sender.send(task).await.map_err(|e| {
            error!("Failed to send download task to worker: {e}");
            e
        })?)
    }
}

impl DownloaderWorker {
    /// Runs the download processing loop.
    ///
    /// This method consumes the worker and should be spawned as a background task.
    /// It will run until the corresponding DownloaderHandle is dropped and
    /// all messages have been processed.
    pub async fn run(mut self) {
        info!("Media downloader actor started.");
        while let Some(DownloadTask {
            task_id,
            url,
            callback,
        }) = self.receiver.recv().await
        {
            debug!("Downloading picture from {url}");
            if let Err(err) = self.process_task(&url, callback).await {
                if let Err(e) = self
                    .msg_sender
                    .send(Message::Err(ErrMsg {
                        r#type: ErrType::DownPicFail { url },
                        task_id,
                        err: err.to_string(),
                    }))
                    .await
                {
                    error!("message send failed, channel broke down: {e}");
                    panic!("message send failed, channel broke down: {e}");
                }
            }
        }
        info!("Media downloader actor finished.");
    }

    async fn process_task(&self, url: &str, callback: AsyncDownloadCallback) -> Result<()> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| {
                error!("Failed to send request when download picture from {url}: {e}");
                e
            })?;
        let body = response.bytes().await.map_err(|e| {
            error!("Failed to read bytes from response for {url}: {e}");
            e
        })?;
        info!("Successfully downloaded picture from {url}");
        (callback)(body).await
    }
}
