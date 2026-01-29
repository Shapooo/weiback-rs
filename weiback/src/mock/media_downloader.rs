//! Test mock for media_downloader
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::mpsc;
use url::Url;

use crate::{
    core::task::TaskContext,
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
        _ctx: Arc<TaskContext>,
        url: &Url,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
        let response = self.inner.lock().unwrap().responses.remove(url);
        match response {
            Some(Ok(data)) => {
                (callback)(data).await?;
                Ok(())
            }
            Some(Err(e)) => Err(e),
            None => {
                if self.default_succ {
                    (callback)(Bytes::from("default media")).await?;
                    Ok(())
                } else {
                    Err(Error::InconsistentTask(format!("URL not mocked: {}", url)))
                }
            }
        }
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
            move |data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                assert_eq!(data, expected_data);
                *callback_executed_clone.lock().unwrap() = true;
                Box::pin(async { Ok(()) })
            },
        );

        let (msg_sender, _msg_recver) = mpsc::channel(20);
        let dummy_context = Arc::new(TaskContext {
            task_id: 0,
            config: Default::default(),
            msg_sender,
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
            move |_data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("callback should not be called");
            },
        );

        let (msg_sender, _msg_recver) = mpsc::channel(20);
        let dummy_context = Arc::new(TaskContext {
            task_id: 0,
            config: Default::default(),
            msg_sender,
        });
        let result = mock_downloader
            .download_media(dummy_context, &url, callback)
            .await;
        assert!(result.is_err());
        match result {
            Err(Error::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            _ => panic!("unexpected error type"),
        }
    }

    #[tokio::test]
    async fn test_download_media_not_mocked() {
        let mock_downloader = MockMediaDownloader::new(false);
        let url = Url::parse("http://example.com/pic.jpg").unwrap();

        let callback = Box::new(
            move |_data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("callback should not be called");
            },
        );

        let (msg_sender, _msg_recver) = mpsc::channel(20);
        let dummy_context = Arc::new(TaskContext {
            task_id: 0,
            config: Default::default(),
            msg_sender,
        });
        let result = mock_downloader
            .download_media(dummy_context, &url, callback)
            .await;
        assert!(result.is_err());
        match result {
            Err(Error::InconsistentTask(msg)) => assert!(msg.contains("URL not mocked")),
            _ => panic!("unexpected error type"),
        }
    }
}
