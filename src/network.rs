use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{debug, error, info, trace, warn};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, IntoUrl, Response, StatusCode,
};
use reqwest_cookie_store::CookieStoreMutex;
use serde_json::{from_value, Value};

use crate::app::models::{LongText, Post, User};
use crate::app::Network;

const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";
const RETRY_COUNT: i32 = 3;

#[derive(Debug, PartialEq, Clone)]
enum AuthStatus {
    UnsignedIn,
    SignedIn,
}

#[derive(Debug)]
pub struct NetworkImpl {
    uid: Option<i64>,
    http_client: Client,
    auth_status: AuthStatus,
}

impl NetworkImpl {
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

        Ok(NetworkImpl {
            uid: None,
            http_client: web_client,
            auth_status: AuthStatus::UnsignedIn,
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
}

impl Network for Arc<NetworkImpl> {
    async fn get_favorite_num(&self) -> Result<u32> {
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
    async fn get_user(&self, id: i64) -> Result<User> {
        let url = User::get_download_url(id);
        let mut json = self.get(url).await?.json::<Value>().await?;
        if json["ok"] != 1 {
            Err(anyhow!("fetch user info failed: {:?}", json))
        } else {
            Ok(from_value(json["data"]["user"].take())?)
        }
    }

    async fn unfavorite_post(&self, id: i64) -> Result<()> {
        let idstr = id.to_string();
        if let Err(err) = self
            .post(
                Post::get_unfavorite_url(),
                &serde_json::json!({ "id": idstr }),
            )
            .await
        {
            error!("unfavorite {id} post failed, because {err}");
        };
        Ok(())
    }
}
