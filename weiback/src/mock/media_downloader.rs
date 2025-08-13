//! Test mock for media_downloader
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use bytes::Bytes;

use crate::{
    error::{Error, Result},
    media_downloader::{AsyncDownloadCallback, MediaDownloader},
};

#[derive(Debug, Clone, Default)]
pub struct MediaDownloaderMock {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    responses: HashMap<String, Result<Bytes>>,
}

impl MediaDownloaderMock {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_response(&self, url: String, response: Result<Bytes>) {
        self.inner.lock().unwrap().responses.insert(url, response);
    }
}

impl MediaDownloader for MediaDownloaderMock {
    async fn download_picture(
        &self,
        _task_id: u64,
        url: String,
        callback: AsyncDownloadCallback,
    ) -> Result<()> {
        let response = self.inner.lock().unwrap().responses.remove(&url);
        match response {
            Some(Ok(data)) => {
                (callback)(data).await?;
                Ok(())
            }
            Some(Err(e)) => Err(e),
            None => Err(Error::InconsistentTask(format!("URL not mocked: {}", url))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[tokio::test]
    async fn test_download_picture_success() {
        let mock_downloader = MediaDownloaderMock::new();
        let url = "http://example.com/pic.jpg".to_string();
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

        let result = mock_downloader.download_picture(1, url, callback).await;
        assert!(result.is_ok());
        assert!(*callback_executed.lock().unwrap());
    }

    #[tokio::test]
    async fn test_download_picture_error() {
        let mock_downloader = MediaDownloaderMock::new();
        let url = "http://example.com/pic.jpg".to_string();
        let error = Error::Io(io::Error::new(io::ErrorKind::NotFound, "not found"));
        mock_downloader.add_response(url.clone(), Err(error));

        let callback = Box::new(
            move |_data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("callback should not be called");
            },
        );

        let result = mock_downloader.download_picture(1, url, callback).await;
        assert!(result.is_err());
        match result {
            Err(Error::Io(e)) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            _ => panic!("unexpected error type"),
        }
    }

    #[tokio::test]
    async fn test_download_picture_not_mocked() {
        let mock_downloader = MediaDownloaderMock::new();
        let url = "http://example.com/pic.jpg".to_string();

        let callback = Box::new(
            move |_data: Bytes| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                panic!("callback should not be called");
            },
        );

        let result = mock_downloader.download_picture(1, url, callback).await;
        assert!(result.is_err());
        match result {
            Err(Error::InconsistentTask(msg)) => assert!(msg.contains("URL not mocked")),
            _ => panic!("unexpected error type"),
        }
    }
}
