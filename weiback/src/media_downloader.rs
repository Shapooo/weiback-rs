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
use super::core::task_manager::{SubTaskError, SubTaskErrorType};
use crate::error::Result;

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
    dyn FnOnce(
            Arc<TaskContext>,
            Bytes,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>
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
            if let Err(err) = self.process_task(ctx.clone(), &url, callback).await {
                let sub_task_err = SubTaskError {
                    error_type: SubTaskErrorType::DownloadMedia(url.to_string()),
                    message: err.to_string(),
                };
                if let Err(e) = ctx.task_manager.add_sub_task_error(sub_task_err) {
                    error!("Failed to add sub-task error: {}", e);
                }
            }
        }
        info!("Media downloader actor finished.");
    }

    async fn process_task(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
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
        (callback)(ctx, body).await
    }
}

#[cfg(test)]
mod local_tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use mockito::Server;
    use tokio::sync::Notify;

    use super::*;
    use crate::core::task_manager::TaskManager;
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
            move |_ctx: Arc<TaskContext>,
                  data: Bytes|
                  -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                let expected_data = Bytes::from(mock_body);
                assert_eq!(data, expected_data);
                callback_executed_clone.store(true, Ordering::SeqCst);
                notify_clone.notify_one();
                Box::pin(async { Ok(()) })
            },
        );

        tokio::spawn(worker.run());

        let dummy_context = Arc::new(TaskContext {
            task_id: Some(1),
            config: Default::default(),
            task_manager: Arc::new(TaskManager::new()),
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
            |_: Arc<TaskContext>, _: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("Callback should not be called on network error");
            },
        );

        tokio::spawn(worker.run());

        let task_manager = Arc::new(TaskManager::new());
        let dummy_context = Arc::new(TaskContext {
            task_id: Some(1),
            config: Default::default(),
            task_manager: task_manager.clone(),
        });
        handle
            .download_media(dummy_context, &url, callback)
            .await
            .unwrap();

        // Allow some time for the worker to process the task
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let errors = task_manager.get_and_clear_sub_task_errors().unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0].error_type {
            SubTaskErrorType::DownloadMedia(err_url) => {
                assert_eq!(err_url, url.as_str());
            }
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
            move |_: Arc<TaskContext>,
                  _: Bytes|
                  -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                Box::pin(async {
                    Err(Error::Io(std::io::Error::from(
                        std::io::ErrorKind::PermissionDenied,
                    )))
                })
            },
        );

        tokio::spawn(worker.run());

        let task_manager = Arc::new(TaskManager::new());
        let dummy_context = Arc::new(TaskContext {
            task_id: Some(1),
            config: Default::default(),
            task_manager: task_manager.clone(),
        });
        handle
            .download_media(dummy_context, &Url::parse(&url).unwrap(), callback)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let errors = task_manager.get_and_clear_sub_task_errors().unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0].error_type {
            SubTaskErrorType::DownloadMedia(err_url) => {
                assert_eq!(Url::parse(err_url), Url::parse(&url));
                assert_eq!(errors[0].message, "I/O error: permission denied");
            }
        }
        mock.assert_async().await;
    }
}
