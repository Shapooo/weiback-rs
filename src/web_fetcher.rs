use std::sync::Arc;

use log::{debug, info, trace, warn};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, IntoUrl, Response, StatusCode,
};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::login::{save_login_info, to_login_info};

const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";
const RETRY_COUNT: i32 = 3;

#[derive(Debug)]
pub struct WebFetcher {
    uid: i64,
    cookie: Arc<CookieStoreMutex>,
    web_client: Client,
    mobile_client: Client,
    pic_client: Client,
}

impl WebFetcher {
    pub fn from_cookies(uid: i64, cookie_store: CookieStore) -> Result<Self> {
        let xsrf_token = cookie_store
            .get("weibo.com", "/", "XSRF-TOKEN")
            .ok_or(Error::Other("xsrf-token-not-found".into()))?
            .value()
            .to_owned();
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));
        let web_headers = HeaderMap::from_iter([
            (
                header::ACCEPT,
                HeaderValue::from_static("application/json, text/plain, */*"),
            ),
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
                HeaderValue::from_static("v2.40.55"),
            ),
            (
                HeaderName::from_static("server-version"),
                HeaderValue::from_static("v2023.05.23.3"),
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

        let mobile_headers = HeaderMap::from_iter([
            (
                header::ACCEPT,
                HeaderValue::from_static(
                    "text/html,application/xhtml+xml,application/xml;\
                     q=0.9,image/webp,image/apng,*/*;q=0.8,\
                     application/signed-exchange;v=b3;q=0.7",
                ),
            ),
            (
                header::USER_AGENT,
                HeaderValue::from_static(
                    "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) \
                     AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 \
                     Mobile/15E148 Safari/604.1 Edg/116.0.0.0",
                ),
            ),
            (
                header::ACCEPT_LANGUAGE,
                HeaderValue::from_static("zh-CN,zh;q=0.9"),
            ),
            (
                header::ACCEPT_ENCODING,
                HeaderValue::from_static("gzip, deflate, br"),
            ),
            (header::DNT, HeaderValue::from_static("1")),
            (
                HeaderName::from_static("sec-fetch-dest"),
                HeaderValue::from_static("document"),
            ),
            (
                HeaderName::from_static("sec-fetch-mode"),
                HeaderValue::from_static("navigate"),
            ),
            (
                HeaderName::from_static("sec-fetch-site"),
                HeaderValue::from_static("none"),
            ),
        ]);
        let mobile_client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(cookie_store.clone())
            .default_headers(mobile_headers)
            .build()?;

        let pic_headers = HeaderMap::from_iter([
        (
            header::USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/113.0",
            ),
        ),
        (
            header::ACCEPT,
            HeaderValue::from_static("image/avif,image/webp,*/*"),
        ),
        (
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.5"),
        ),
        (
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br"),
        ),
        (header::DNT, HeaderValue::from_static("1")),
        (header::CONNECTION, HeaderValue::from_static("keep-alive")),
        (
            header::REFERER,
            HeaderValue::from_static("https://weibo.com/"),
        ),
        (header::TE, HeaderValue::from_static("trailers")),
        (HeaderName::from_static("sec-fetch-dest"), HeaderValue::from_static("image")),
        (HeaderName::from_static("sec-fetch-mode"), HeaderValue::from_static("no-cors")),
        (HeaderName::from_static("sec-fetch-site"), HeaderValue::from_static("cross-site"))]);
        let pic_client = reqwest::Client::builder()
            .default_headers(pic_headers)
            .build()?;

        Ok(WebFetcher {
            uid,
            cookie: cookie_store.clone(),
            web_client,
            mobile_client,
            pic_client,
        })
    }

    pub async fn post(&self, url: impl IntoUrl, client: &Client, body: &Value) -> Result<Response> {
        let request = client.post(url).json(body).build()?;
        self._http_common(request, client).await
    }

    pub async fn get(&self, url: impl IntoUrl, client: &Client) -> Result<Response> {
        let request = client.get(url).build()?;
        self._http_common(request, client).await
    }

    async fn _http_common(&self, request: reqwest::Request, client: &Client) -> Result<Response> {
        let url_str = request.url().as_str();
        let mut status_code = StatusCode::OK;
        for _ in 0..RETRY_COUNT {
            let res = client.execute(request.try_clone().unwrap()).await?;
            if res.status().is_success() {
                return Ok(res);
            } else if res.status().is_client_error() {
                status_code = res.status();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Err(Error::ResourceGetFailed(format!(
            "http request {} failed with status code {}",
            url_str, status_code
        )))
    }

    pub fn mobile_client(&self) -> &Client {
        &self.mobile_client
    }

    pub fn web_client(&self) -> &Client {
        &self.web_client
    }

    pub fn pic_client(&self) -> &Client {
        &self.pic_client
    }

    pub async fn fetch_fav_total_num(&self) -> Result<u32> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let ret_json: Value = self
            .get(FAVORITES_TAGS_API, &self.web_client)
            .await?
            .json()
            .await?;
        trace!("get fav tag data: {:?}", ret_json);
        if ret_json["ok"] != 1 {
            Err(Error::ResourceGetFailed(format!(
                "fav total num get failed: {:?}",
                ret_json
            )))
        } else {
            ret_json["fav_total_num"]
                .as_u64()
                .ok_or(Error::MalFormat(format!(
                    "no fav_total_num field in response: {:?}",
                    ret_json
                )))
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
