//! This module provides a mock implementation of the [`MediaDownloader`] trait.
//!
//! `MockMediaDownloader` is used in tests to simulate media download operations.
//! It can be pre-configured with specific responses for given URLs, or default to
//! success/failure based on its `default_succ` setting. This allows testing
//! components that depend on media downloads without actual network interaction.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use url::Url;

use crate::{
    core::{
        task::TaskContext,
        task_manager::{SubTaskError, SubTaskErrorType, TaskManager},
    },
    error::{Error, Result},
    media_downloader::{AsyncDownloadCallback, MediaDownloader},
};

/// A mock implementation of the [`MediaDownloader`] trait.
///
/// This struct allows tests to control the outcomes of media download requests.
#[derive(Debug, Clone, Default)]
pub struct MockMediaDownloader {
    inner: Arc<Mutex<Inner>>,
    /// If `true`, any un-mocked download request will succeed with dummy data.
    /// If `false`, it will fail.
    default_succ: bool,
}

/// Internal state for `MockMediaDownloader`.
#[derive(Debug, Default)]
struct Inner {
    /// A map from URL to a pre-defined download result (either success with `Bytes` or an `Error`).
    responses: HashMap<Url, Result<Bytes>>,
}

impl MockMediaDownloader {
    /// Creates a new `MockMediaDownloader` instance.
    ///
    /// # Arguments
    /// * `default_succ` - Determines the behavior for URLs that are not explicitly mocked.
    pub fn new(default_succ: bool) -> Self {
        Self {
            inner: Default::default(),
            default_succ,
        }
    }

    /// Adds a pre-defined response for a specific URL.
    ///
    /// When `download_media` is called with this `url`, it will return the provided `response`.
    ///
    /// # Arguments
    /// * `url` - The URL to mock.
    /// * `response` - The `Result<Bytes>` that `download_media` should return for this URL.
    pub fn add_response(&self, url: Url, response: Result<Bytes>) {
        self.inner.lock().unwrap().responses.insert(url, response);
    }
}

impl MediaDownloader for MockMediaDownloader {
    /// Simulates downloading a media file.
    ///
    /// It checks for a mocked response for the given URL. If found, it uses that.
    /// Otherwise, its behavior depends on `default_succ`. If `default_succ` is true,
    /// it calls the callback with dummy data; if false, it records a sub-task error.
    ///
    /// # Arguments
    /// * `ctx` - The task context for reporting errors.
    /// * `url` - The URL of the media to "download".
    /// * `callback` - The callback to execute with the "downloaded" data.
    ///
    /// # Returns
    /// A `Result` indicating if the download request was processed by the mock.
    /// Actual success/failure of the simulated download is handled by the mock's
    /// internal logic and reported via `ctx.task_manager.add_sub_task_error`.
    async fn download_media(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
        let response = self.inner.lock().unwrap().responses.remove(url);
        let result = match response {
            Some(Ok(data)) => (callback)(ctx.clone(), data).await,
            Some(Err(e)) => Err(e),
            None => {
                if self.default_succ {
                    (callback)(ctx.clone(), Bytes::from("default media")).await
                } else {
                    Err(Error::InconsistentTask(format!("URL not mocked: {}", url)))
                }
            }
        };

        if let Err(err) = result {
            let sub_task_err = SubTaskError {
                error_type: SubTaskErrorType::DownloadMedia(url.to_string()),
                message: err.to_string(),
            };
            ctx.task_manager.add_sub_task_error(sub_task_err)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use std::io;

    #[tokio::test]
    async fn test_download_media_success() {
        let mock_downloader = MockMediaDownloader::new(true);
        let url = Url::parse("http://example.com/pic.jpg").unwrap();
        let expected_data = Bytes::from_static(b"picture data");
        mock_downloader.add_response(url.clone(), Ok(expected_data.clone()));

        let callback_executed = Arc::new(Mutex::new(false));
        let callback_executed_clone = callback_executed.clone();

        let callback = Box::new(
            move |_, data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                assert_eq!(data, expected_data);
                *callback_executed_clone.lock().unwrap() = true;
                Box::pin(async { Ok(()) })
            },
        );

        let task_manager = Arc::new(TaskManager::new());
        let dummy_context = Arc::new(TaskContext {
            task_id: Some(0),
            config: Default::default(),
            task_manager,
        });
        let result = mock_downloader
            .download_media(dummy_context, &url, callback)
            .await;
        assert!(result.is_ok());
        assert!(*callback_executed.lock().unwrap());
    }

    #[tokio::test]
    async fn test_download_media_error() {
        let mock_downloader = MockMediaDownloader::new(true);
        let url = Url::parse("http://example.com/pic.jpg").unwrap();
        let error = Error::Io(io::Error::new(io::ErrorKind::NotFound, "not found"));
        mock_downloader.add_response(url.clone(), Err(error));

        let callback = Box::new(
            move |_, _data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("callback should not be called");
            },
        );

        let task_manager = Arc::new(TaskManager::new());
        let dummy_context = Arc::new(TaskContext {
            task_id: Some(0),
            config: Default::default(),
            task_manager: task_manager.clone(),
        });
        let result = mock_downloader
            .download_media(dummy_context, &url, callback)
            .await;
        assert!(result.is_ok());

        let errors = task_manager.get_and_clear_sub_task_errors().unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0].error_type {
            SubTaskErrorType::DownloadMedia(err_url) => {
                assert_eq!(*err_url, url.to_string());
                assert!(errors[0].message.contains("not found"));
            }
        }
    }

    #[tokio::test]
    async fn test_download_media_not_mocked() {
        let mock_downloader = MockMediaDownloader::new(false);
        let url = Url::parse("http://example.com/pic.jpg").unwrap();

        let callback = Box::new(
            move |_, _data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("callback should not be called");
            },
        );

        let task_manager = Arc::new(TaskManager::new());
        let dummy_context = Arc::new(TaskContext {
            task_id: Some(0),
            config: Default::default(),
            task_manager: task_manager.clone(),
        });
        let result = mock_downloader
            .download_media(dummy_context, &url, callback)
            .await;
        assert!(result.is_ok());

        let errors = task_manager.get_and_clear_sub_task_errors().unwrap();
        assert_eq!(errors.len(), 1);
        match &errors[0].error_type {
            SubTaskErrorType::DownloadMedia(err_url) => {
                assert_eq!(*err_url, url.to_string());
                assert!(errors[0].message.contains("URL not mocked"));
            }
        }
    }
}
