//! This module provides an asynchronous media downloader implemented using an
//! actor-based pattern.
//!
//! The downloader is split into two parts:
//! 1.  [`MediaDownloaderHandle`]: A thread-safe handle used to queue download requests
//!     via a message channel.
//! 2.  [`DownloaderWorker`]: A background task that processes these requests, performs
//!     HTTP downloads using `reqwest`, and executes callbacks upon success.
//!
//! This architecture ensures that media downloads (which can be slow or unreliable)
//! do not block the main application flow and can be easily monitored.
//!
//! ## Concurrency
//!
//! The downloader supports concurrent downloads (up to [`MAX_CONCURRENT_DOWNLOADS`]
//! simultaneous downloads) using `tokio::select!` with `FuturesUnordered` to multiplex
//! between multiple in-flight download tasks.

#![allow(async_fn_in_trait)]
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use url::Url;

use super::core::task::TaskContext;
use super::core::task_manager::{TaskError, TaskErrorType};
use crate::error::Result;

/// Maximum number of concurrent downloads.
pub const MAX_CONCURRENT_DOWNLOADS: usize = 5;

/// The status of the media downloader.
#[derive(Debug, Clone, Serialize)]
pub struct DownloaderStatus {
    /// URLs currently being downloaded (up to MAX_CONCURRENT_DOWNLOADS).
    pub active_downloads: Vec<String>,
    /// Number of items waiting in the queue.
    pub queue_length: usize,
}

/// A trait for receiving downloader status updates.
pub trait MediaDownloaderStatusListener: Send + Sync {
    /// Called when the downloader status changes.
    fn on_status_updated(&self, status: &DownloaderStatus);
}

/// A trait for high-level media downloading capabilities.
pub trait MediaDownloader: Clone + Send + Sync + 'static {
    /// Queues a media download request.
    ///
    /// # Arguments
    /// * `ctx` - The task context for progress and error reporting.
    /// * `url` - The URL of the media file to download.
    /// * `callback` - An async closure executed on the downloaded data if successful.
    async fn download_media(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
        callback: AsyncDownloadCallback,
    ) -> Result<()>;
}

/// A type alias for the asynchronous callback function executed after a successful download.
pub type AsyncDownloadCallback = Box<
    dyn FnOnce(
            Arc<TaskContext>,
            Bytes,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>
        + Send
        + 'static,
>;

/// Internal structure representing a single download request sent to the worker.
struct DownloadTask {
    ctx: Arc<TaskContext>,
    url: Url,
    callback: AsyncDownloadCallback,
}

/// Internal state for tracking downloader status.
#[derive(Debug)]
pub struct DownloaderStatusState {
    /// URLs of currently active downloads.
    pub active_downloads: Mutex<Vec<String>>,
    /// Number of items waiting in the queue.
    pub queue_length: AtomicUsize,
}

impl DownloaderStatusState {
    fn new() -> Self {
        Self {
            active_downloads: Mutex::new(Vec::new()),
            queue_length: AtomicUsize::new(0),
        }
    }

    fn get_status(&self) -> DownloaderStatus {
        DownloaderStatus {
            active_downloads: self.active_downloads.lock().unwrap().clone(),
            queue_length: self.queue_length.load(Ordering::Relaxed),
        }
    }
}

/// The worker that runs in a background task and performs actual downloads.
#[must_use = "The worker must be spawned to process download requests"]
pub struct DownloaderWorker {
    receiver: mpsc::Receiver<DownloadTask>,
    client: Client,
    status_listener: Arc<Mutex<Option<Box<dyn MediaDownloaderStatusListener>>>>,
    status: Arc<DownloaderStatusState>,
}

/// A thread-safe handle for communicating with the [`DownloaderWorker`].
#[derive(Clone)]
pub struct MediaDownloaderHandle {
    sender: mpsc::Sender<DownloadTask>,
    status: Arc<DownloaderStatusState>,
    status_listener: Arc<Mutex<Option<Box<dyn MediaDownloaderStatusListener>>>>,
}

/// Initializes a new media downloader system.
///
/// # Arguments
/// * `buffer` - The capacity of the message channel.
/// * `client` - The HTTP client used for downloads.
///
/// # Returns
/// A tuple containing the handle and the worker. The worker **must** be spawned
/// into a task (e.g., using `tokio::spawn(worker.run())`) to function.
pub fn create_downloader(
    buffer: usize,
    client: Client,
) -> (MediaDownloaderHandle, DownloaderWorker) {
    let (sender, receiver) = mpsc::channel(buffer);
    let status = Arc::new(DownloaderStatusState::new());
    let status_listener = Arc::new(Mutex::new(None));

    let handle = MediaDownloaderHandle {
        sender,
        status: status.clone(),
        status_listener: status_listener.clone(),
    };
    let worker = DownloaderWorker {
        receiver,
        client,
        status_listener,
        status,
    };
    (handle, worker)
}

impl MediaDownloader for MediaDownloaderHandle {
    /// Sends a download request to the background worker.
    ///
    /// This method is non-blocking and returns as soon as the request is queued.
    ///
    /// # Errors
    /// Returns an error if the internal channel is closed.
    #[tracing::instrument(skip(self, ctx, callback), fields(url = %url))]
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
        self.status.queue_length.fetch_add(1, Ordering::Relaxed);
        self.notify_status();
        Ok(self.sender.send(task).await.inspect_err(|e| {
            error!("Failed to send download task to worker: {e}");
        })?)
    }
}

impl MediaDownloaderHandle {
    /// Sets a listener for downloader status updates.
    pub fn set_status_listener(&self, listener: Box<dyn MediaDownloaderStatusListener>) {
        let mut guard = self.status_listener.lock().unwrap();
        *guard = Some(listener);
    }

    /// Notifies the listener of the current status.
    fn notify_status(&self) {
        if let Some(listener) = self.status_listener.lock().unwrap().as_ref() {
            listener.on_status_updated(&self.status.get_status());
        }
    }
}

impl DownloaderWorker {
    /// Sets a listener for downloader status updates.
    ///
    /// This should be called before spawning the worker.
    pub fn set_status_listener(&mut self, listener: Box<dyn MediaDownloaderStatusListener>) {
        let mut guard = self.status_listener.lock().unwrap();
        *guard = Some(listener);
    }

    /// Starts the worker's processing loop.
    ///
    /// This method will run indefinitely until the handle is dropped or the channel
    /// is closed. It should be spawned onto a background executor.
    pub async fn run(mut self) {
        info!("Media downloader actor started.");
        let mut workers: FuturesUnordered<JoinHandle<(String, Result<()>)>> =
            FuturesUnordered::new();

        loop {
            tokio::select! {
                // 只有当 workers 数量小于上限时，才去 poll receiver
                opt = self.receiver.recv(), if workers.len() < MAX_CONCURRENT_DOWNLOADS => {
                    match opt {
                        Some(task) => {
                            self.status.queue_length.fetch_sub(1, Ordering::Relaxed);
                            let url = task.url.to_string();
                            let ctx = task.ctx.clone();
                            let client = self.client.clone();
                            let url_for_spawn = url.clone();

                            // 更新活跃列表
                            {
                                let mut active = self.status.active_downloads.lock().unwrap();
                                active.push(url.clone());
                            }
                            self.notify_status();

                            debug!("Starting download: {url}");
                            workers.push(tokio::spawn(async move {
                                let result = Self::do_process_task(&client, ctx.clone(), &url_for_spawn, task.callback).await;

                                if let Err(err) = &result {
                                    let task_err = TaskError {
                                        error_type: TaskErrorType::DownloadMedia(url_for_spawn.clone()),
                                        message: err.to_string(),
                                    };
                                    if let Err(e) = ctx.task_manager.report_task_error(task_err) {
                                        error!("Failed to add task error: {}", e);
                                    }
                                }

                                (url_for_spawn, result)
                            }));
                        }
                        None => {
                            // channel 关了
                            if workers.is_empty() {
                                break;
                            }
                        }
                    }
                }

                // 监听已有的任务完成
                Some(res) = workers.next(), if !workers.is_empty() => {
                    // 某个任务结束了，从活跃列表移除
                    if let Ok((url, _)) = res {
                        let mut active = self.status.active_downloads.lock().unwrap();
                        active.retain(|u| u != &url);
                    }
                    self.notify_status();
                }
            }
        }

        info!("Media downloader actor finished.");
    }

    /// Notifies the listener of the current status.
    fn notify_status(&self) {
        if let Some(listener) = self.status_listener.lock().unwrap().as_ref() {
            listener.on_status_updated(&self.status.get_status());
        }
    }

    /// Performs the HTTP request and handles the response.
    #[tracing::instrument(skip(client, ctx, callback), fields(url = %url))]
    async fn do_process_task(
        client: &Client,
        ctx: Arc<TaskContext>,
        url: &str,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
        let url = Url::parse(url).inspect_err(|e| {
            error!("Failed to parse URL {url}: {e}");
        })?;
        let response = client
            .get(url.clone())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .inspect_err(|e| {
                error!("Failed to send request when download media file from {url}: {e}");
            })?;
        let body = response.bytes().await.inspect_err(|e| {
            error!("Failed to read bytes from response for {url}: {e}");
        })?;
        debug!("Successfully downloaded media file from {url}");
        (callback)(ctx, body).await
    }
}

use tokio::task::JoinHandle;

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

        let errors = task_manager.get_and_clear_task_errors().unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0].error_type {
            TaskErrorType::DownloadMedia(err_url) => {
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

        let errors = task_manager.get_and_clear_task_errors().unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0].error_type {
            TaskErrorType::DownloadMedia(err_url) => {
                assert_eq!(Url::parse(err_url), Url::parse(&url));
                assert_eq!(errors[0].message, "I/O error: permission denied");
            }
        }
        mock.assert_async().await;
    }
}
