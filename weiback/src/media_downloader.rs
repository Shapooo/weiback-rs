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

/// A media downloader that handles downloading pictures in a separate actor.
#[derive(Clone, Debug)]
pub struct MediaDownloaderImpl {
    sender: mpsc::Sender<DownloadTask>,
}

impl MediaDownloaderImpl {
    /// Creates a new `MediaDownloader` and spawns the background downloader actor.
    ///
    /// # Arguments
    ///
    /// * `client` - A `reqwest::Client` to be used for making HTTP requests.
    pub fn new(client: Client, message_sender: mpsc::Sender<Message>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<DownloadTask>(500);

        tokio::spawn(async move {
            // TODO: spawn in sync function
            info!("Media downloader actor started.");
            while let Some(DownloadTask {
                task_id,
                url,
                callback,
            }) = receiver.recv().await
            {
                debug!("Downloading picture from {url}");
                let res = worker(&client, &url, callback).await;
                if let Err(err) = res {
                    if let Err(e) = message_sender
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
        });

        Self { sender }
    }
}

async fn worker(client: &Client, url: &str, callback: AsyncDownloadCallback) -> Result<()> {
    let response = client
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

impl MediaDownloader for MediaDownloaderImpl {
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
