use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::warn;
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, Response, StatusCode,
};
use reqwest_cookie_store::CookieStoreMutex;
use serde_json::Value;

const RETRY_COUNT: i32 = 3;

#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: Arc<Client>,
}

impl HttpClient {
    pub fn new(cookie_store: Arc<CookieStoreMutex>) -> Result<Self> {
        let web_headers = HeaderMap::from_iter([
            (header::ACCEPT, HeaderValue::from_static("*/*")),
            (
                header::REFERER,
                HeaderValue::from_static("https://weibo.com/"),
            ),
            (
                header::USER_AGENT,
                HeaderValue::from_static(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) \
                     Gecko/20100101 Firefox/113.0",
                ),
            ),
            (
                header::ACCEPT_LANGUAGE,
                HeaderValue::from_static("en-US,en;q=0.5"),
            ),
            (
                header::ACCEPT_ENCODING,
                HeaderValue::from_static("gzip, deflate, br"),
            ),
            (
                HeaderName::from_static("x-requested-with"),
                HeaderValue::from_static("XMLHttpRequest"),
            ),
            (
                HeaderName::from_static("client-version"),
                HeaderValue::from_static("v2.44.47"),
            ),
            (
                HeaderName::from_static("server-version"),
                HeaderValue::from_static("v2024.01.08.1"),
            ),
            (header::DNT, HeaderValue::from_static("1")),
            (header::CONNECTION, HeaderValue::from_static("keep-alive")),
            (
                HeaderName::from_static("sec-fetch-dest"),
                HeaderValue::from_static("empty"),
            ),
            (
                HeaderName::from_static("sec-fetch-mode"),
                HeaderValue::from_static("cors"),
            ),
            (
                HeaderName::from_static("sec-fetch-site"),
                HeaderValue::from_static("same-origin"),
            ),
            (header::PRAGMA, HeaderValue::from_static("no-cache")),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            (header::TE, HeaderValue::from_static("trailers")),
        ]);

        let web_client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(cookie_store)
            .default_headers(web_headers)
            .build()?;

        Ok(Self {
            inner: Arc::new(web_client),
        })
    }

    async fn _http_common(&self, request: reqwest::Request) -> Result<Response> {
        let url_str = request.url().as_str();
        let mut status_code = StatusCode::OK;
        for _ in 0..RETRY_COUNT {
            let res = self.inner.execute(request.try_clone().unwrap()).await?;
            if res.status().is_success() {
                return Ok(res);
            } else if res.status().is_client_error() {
                status_code = res.status();
            }
            warn!(
                "http request {} failed with status code {}, start to retry",
                url_str, status_code
            );
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Err(anyhow!(
            "http request {} failed with status code {} finally",
            url_str,
            status_code
        ))
    }

    pub async fn post(&self, url: &str, body: &Value) -> Result<Response> {
        let request = self.inner.post(url).json(body).build()?;
        self._http_common(request).await
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        let request = self.inner.get(url).build()?;
        self._http_common(request).await
    }
}
