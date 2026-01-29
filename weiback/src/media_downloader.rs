#![allow(async_fn_in_trait)]
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, error, info};
use reqwest::Client;
use tokio::sync::mpsc;
use url::Url;

use super::core::task::TaskContext;
use crate::error::Result;
use crate::message::{ErrMsg, ErrType, Message};

pub trait MediaDownloader: Clone + Send + Sync + 'static {
    async fn download_media(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
        callback: AsyncDownloadCallback,
    ) -> Result<()>;
}

/// The callback is for success cases and is async.
pub type AsyncDownloadCallback = Box<
    dyn FnOnce(Bytes) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>
        + Send
        + 'static,
>;

/// A task for the media downloader actor.
struct DownloadTask {
    ctx: Arc<TaskContext>,
    url: Url,
    callback: AsyncDownloadCallback,
}

/// The worker that runs in a background task.
/// It owns the receiver and the HTTP client,
/// and is consumed when its `run` method is called.
#[must_use = "The worker must be spawned to process download requests"]
pub struct DownloaderWorker {
    receiver: mpsc::Receiver<DownloadTask>,
    client: Client,
}

/// A media downloader that handles downloading media file in a separate actor.
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
) -> (MediaDownloaderHandle, DownloaderWorker) {
    let (sender, receiver) = mpsc::channel(buffer);
    let handle = MediaDownloaderHandle { sender };
    let worker = DownloaderWorker { receiver, client };
    (handle, worker)
}

impl MediaDownloader for MediaDownloaderHandle {
    /// Queues a media file for download.
    ///
    /// This method sends a task to the background downloader actor and returns immediately.
    /// The provided async callback will be executed once the download is complete and successful.
    /// If the download fails, the task is discarded and the callback is never called.
    async fn download_media(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
        let task = DownloadTask {
            ctx,
            url: url.to_owned(),
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
        while let Some(DownloadTask { ctx, url, callback }) = self.receiver.recv().await {
            debug!("Downloading media from {url}");
            if let Err(err) = self.process_task(&url, callback).await
                && let Err(e) = ctx
                    .msg_sender
                    .send(Message::Err(ErrMsg {
                        r#type: ErrType::DownMediaFail {
                            url: url.to_string(),
                        },
                        task_id: ctx.task_id,
                        err: err.to_string(),
                    }))
                    .await
            {
                error!("message send failed, channel broke down: {e}");
                panic!("message send failed, channel broke down: {e}");
            }
        }
        info!("Media downloader actor finished.");
    }

    async fn process_task(&self, url: &Url, callback: AsyncDownloadCallback) -> Result<()> {
        let response = self
            .client
            .get(url.to_owned())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| {
                error!("Failed to send request when download media file from {url}: {e}");
                e
            })?;
        let body = response.bytes().await.map_err(|e| {
            error!("Failed to read bytes from response for {url}: {e}");
            e
        })?;
        debug!("Successfully downloaded media file from {url}");
        (callback)(body).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use mockito::Server;
    use tokio::sync::Notify;

    use super::*;
    use crate::error::Error;

    #[tokio::test]
    async fn test_download_media_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock_body = "picture data";
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(mock_body)
            .create_async()
            .await;

        let client = Client::new();
        let (handle, worker) = create_downloader(1, client);

        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let callback_executed = Arc::new(AtomicBool::new(false));
        let callback_executed_clone = callback_executed.clone();

        let callback = Box::new(
            move |data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                let expected_data = Bytes::from(mock_body);
                assert_eq!(data, expected_data);
                callback_executed_clone.store(true, Ordering::SeqCst);
                notify_clone.notify_one();
                Box::pin(async { Ok(()) })
            },
        );

        tokio::spawn(worker.run());

        let (msg_sender, _) = mpsc::channel(20);
        let dummy_context = Arc::new(TaskContext {
            task_id: 1,
            config: Default::default(),
            msg_sender,
        });
        handle
            .download_media(dummy_context, &Url::parse(&url).unwrap(), callback)
            .await
            .unwrap();

        notify.notified().await;
        assert!(callback_executed.load(Ordering::SeqCst));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_download_media_network_error() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let url = Url::parse(&url).unwrap();
        let mock = server
            .mock("GET", "/")
            .with_status(404)
            .create_async()
            .await;

        let client = Client::new();
        let (handle, worker) = create_downloader(1, client);

        let callback = Box::new(
            |_: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("Callback should not be called on network error");
            },
        );

        tokio::spawn(worker.run());

        let (msg_sender, mut msg_receiver) = mpsc::channel(20);
        let dummy_context = Arc::new(TaskContext {
            task_id: 1,
            config: Default::default(),
            msg_sender,
        });
        handle
            .download_media(dummy_context, &url, callback)
            .await
            .unwrap();

        let received_msg = msg_receiver.recv().await.unwrap();
        match received_msg {
            Message::Err(ErrMsg {
                r#type: ErrType::DownMediaFail { url: err_url },
                task_id,
                ..
            }) => {
                assert_eq!(err_url, url.as_str());
                assert_eq!(task_id, 1);
            }
            _ => panic!("Expected an error message"),
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_download_media_callback_error() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let mock_body = "picture data";
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(mock_body)
            .create_async()
            .await;

        let client = Client::new();
        let (handle, worker) = create_downloader(1, client);

        let callback = Box::new(
            move |_: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                Box::pin(async {
                    Err(Error::Io(std::io::Error::from(
                        std::io::ErrorKind::PermissionDenied,
                    )))
                })
            },
        );

        tokio::spawn(worker.run());

        let (msg_sender, mut msg_receiver) = mpsc::channel(20);
        let dummy_context = Arc::new(TaskContext {
            task_id: 1,
            config: Default::default(),
            msg_sender,
        });
        handle
            .download_media(dummy_context, &Url::parse(&url).unwrap(), callback)
            .await
            .unwrap();

        let received_msg = msg_receiver.recv().await.unwrap();
        match received_msg {
            Message::Err(ErrMsg {
                r#type: ErrType::DownMediaFail { url: err_url },
                task_id,
                err,
            }) => {
                assert_eq!(Url::parse(&err_url), Url::parse(&url));
                assert_eq!(task_id, 1);
                assert_eq!(err, "I/O error: permission denied");
            }
            _ => panic!("Expected an error message"),
        }
        mock.assert_async().await;
    }
}
