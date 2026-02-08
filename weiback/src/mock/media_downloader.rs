//! Test mock for media_downloader
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

#[derive(Debug, Clone, Default)]
pub struct MockMediaDownloader {
    inner: Arc<Mutex<Inner>>,
    default_succ: bool,
}

#[derive(Debug, Default)]
struct Inner {
    responses: HashMap<Url, Result<Bytes>>,
}

impl MockMediaDownloader {
    pub fn new(default_succ: bool) -> Self {
        Self {
            inner: Default::default(),
            default_succ,
        }
    }

    pub fn add_response(&self, url: Url, response: Result<Bytes>) {
        self.inner.lock().unwrap().responses.insert(url, response);
    }
}

impl MediaDownloader for MockMediaDownloader {
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
