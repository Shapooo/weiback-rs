#![allow(async_fn_in_trait)]
use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use log::{error, info};
use reqwest::Client;
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::message::Message;

pub trait MediaDownloader {
    async fn download_picture(&self, url: String, callback: AsyncDownloadCallback) -> Result<()>;
}

// The callback is for success cases and is async.
type AsyncDownloadCallback = Box<
    dyn FnOnce(Bytes) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>
        + Send
        + 'static,
>;

/// A task for the media downloader actor.
struct DownloadTask {
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
    pub fn new(client: Client, message_sender: mpsc::Sender<Result<Message>>) -> Self {
        // A channel to send download tasks to the downloader actor.
        let (sender, mut receiver) = mpsc::channel::<DownloadTask>(100); // Buffer size of 100

        // TODO: return err msg properly
        tokio::spawn(async move {
            info!("Media downloader actor started.");
            while let Some(task) = receiver.recv().await {
                info!("Downloading picture from {}", &task.url);
                // Use the provided client to make the request
                let res = match client.get(&task.url).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            error!(
                                "Download failed for {}: status code {}",
                                &task.url,
                                response.status()
                            );
                            // TODO: Report this error back via a dedicated error channel.
                            continue; // Discard task on error
                        }
                        match response.bytes().await {
                            Ok(bytes) => {
                                // Download successful, execute the async callback.
                                (task.callback)(bytes).await
                            }
                            Err(e) => {
                                error!(
                                    "Failed to read bytes from response for {}: {}",
                                    &task.url, e
                                );
                                Err(Error::Other(e.to_string()))
                                // TODO: Report this error back via a dedicated error channel.
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to download picture from {}: {}", &task.url, e);
                        // TODO: Report this error back via a dedicated error channel.
                        Err(Error::Other(e.to_string()))
                    }
                };
                if let Err(e) = res {
                    message_sender.send(Err(e)).await.unwrap();
                }
            }
            info!("Media downloader actor finished.");
        });

        Self { sender }
    }
}

impl MediaDownloader for MediaDownloaderImpl {
    /// Queues a picture for download.
    ///
    /// This method sends a task to the background downloader actor and returns immediately.
    /// The provided async callback will be executed once the download is complete and successful.
    /// If the download fails, the task is discarded and the callback is never called.
    async fn download_picture(&self, url: String, callback: AsyncDownloadCallback) -> Result<()> {
        let task = DownloadTask { url, callback };
        self.sender.send(task).await.map_err(|e| {
            error!("Failed to send download task to worker: {e}");
            Error::Other("Media downloader channel has been closed".to_string())
        })
    }
}
