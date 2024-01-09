use crate::login::{save_login_info, to_login_info};

use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{debug, info, trace, warn};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, IntoUrl, Response, StatusCode,
};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde_json::Value;

const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";
const RETRY_COUNT: i32 = 3;

#[derive(Debug)]
pub struct WebFetcher {
    uid: i64,
    cookie: Arc<CookieStoreMutex>,
    http_client: Client,
}

impl WebFetcher {
    pub fn from_cookies(uid: i64, cookie_store: CookieStore) -> Result<Self> {
        let xsrf_token = cookie_store
            .get("weibo.com", "/", "XSRF-TOKEN")
            .ok_or(anyhow!("xsrf-token-not-found"))?
            .value()
            .to_owned();
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));
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
            (
                HeaderName::from_static("x-xsrf-token"),
                HeaderValue::from_str(xsrf_token.as_str())?,
            ),
            (header::PRAGMA, HeaderValue::from_static("no-cache")),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            (header::TE, HeaderValue::from_static("trailers")),
        ]);

        let web_client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(cookie_store.clone())
            .default_headers(web_headers)
            .build()?;

        Ok(WebFetcher {
            uid,
            cookie: cookie_store.clone(),
            http_client: web_client,
        })
    }

    pub async fn post(&self, url: impl IntoUrl, body: &Value) -> Result<Response> {
        let request = self.http_client.post(url).json(body).build()?;
        self._http_common(request).await
    }

    pub async fn get(&self, url: impl IntoUrl) -> Result<Response> {
        let request = self.http_client.get(url).build()?;
        self._http_common(request).await
    }

    async fn _http_common(&self, request: reqwest::Request) -> Result<Response> {
        let url_str = request.url().as_str();
        let mut status_code = StatusCode::OK;
        for _ in 0..RETRY_COUNT {
            let res = self
                .http_client
                .execute(request.try_clone().unwrap())
                .await?;
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

    pub async fn fetch_fav_total_num(&self) -> Result<u32> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let ret_json: Value = self.get(FAVORITES_TAGS_API).await?.json().await?;
        trace!("get fav tag data: {:?}", ret_json);
        if ret_json["ok"] != 1 {
            Err(anyhow!("fav total num get failed: {:?}", ret_json))
        } else {
            ret_json["fav_total_num"]
                .as_u64()
                .ok_or(anyhow!(
                    "no fav_total_num field in response: {:?}",
                    ret_json
                ))
                .map(|v| v as u32)
        }
    }
}

impl Drop for WebFetcher {
    fn drop(&mut self) {
        self.cookie
            .lock()
            .map_or(
                Err(anyhow::anyhow!(
                    "PoisonError: cannot lock Arc<MutexCookieStore>"
                )),
                |mutex_cookie_store| to_login_info(self.uid, &mutex_cookie_store),
            )
            .map(|login_info| save_login_info(&login_info))
            .map_or_else(
                |err| warn!("when save cookie, raise {}", err),
                |_| info!("login_info saved succ"),
            );
    }
}
