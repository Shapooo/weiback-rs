use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, info, trace, warn};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, IntoUrl, Response,
};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde_json::Value;

use crate::data::{LongText, Posts};
use crate::error::{Error, Result};
use crate::login::{save_login_info, to_login_info};

const STATUSES_CONFIG_API: &str = "https://weibo.com/ajax/statuses/config";
const STATUSES_LONGTEXT_API: &str = "https://weibo.com/ajax/statuses/longtext";
const FAVORITES_ALL_FAV_API: &str = "https://weibo.com/ajax/favorites/all_fav";
const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";

#[derive(Debug)]
pub struct WebFetcher {
    uid: &'static str,
    cookie: Arc<CookieStoreMutex>,
    web_client: Client,
    pic_client: Client,
}

impl WebFetcher {
    pub fn from_cookies(uid: &'static str, cookie_store: CookieStore) -> Result<Self> {
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
            pic_client,
        })
    }

    pub async fn unfavorite_post(&self, id: i64) -> Result<()> {
        let id = id.to_string();
        let response = self
            .web_client
            .post("https://weibo.com/ajax/statuses/destoryFavorites")
            .json(&serde_json::json!({ "id": id }))
            .send()
            .await?;
        let status_code = response.status();
        if status_code.as_u16() == 200 {
            Ok(())
        } else {
            let res_json = response.json::<Value>().await?;
            if status_code.as_u16() == 400 && res_json["message"] == "not your collection!" {
                warn!("post {id} have been unfavorited, there may be bugs in code...");
                Ok(())
            } else {
                Err(Error::Other(format!(
                    "unfavorite post get {}: {:?}",
                    status_code.as_u16(),
                    res_json["message"],
                )))
            }
        }
    }

    async fn _fetch(&self, url: impl IntoUrl, client: &Client) -> Result<Response> {
        let url_str = url.as_str().to_owned();
        let res = client.get(url).send().await?;
        if res.status() != 200 {
            Err(Error::ResourceGetFailed(format!(
                "fetch {} failed with status code {}",
                url_str,
                res.status()
            )))
        } else {
            Ok(res)
        }
    }

    pub async fn fetch_posts_meta(&self, uid: &str, page: u32) -> Result<Posts> {
        let url = format!("{FAVORITES_ALL_FAV_API}?uid={uid}&page={page}");
        debug!("fetch meta page, url: {url}");
        let mut posts: Value = self._fetch(url, &self.web_client).await?.json().await?;
        trace!("get json: {posts:?}");
        if posts["ok"] != 1 {
            Err(Error::ResourceGetFailed(format!(
                "fetched data is not ok: {posts:?}"
            )))
        } else if let Value::Array(v) = posts["data"].take() {
            Ok(v)
        } else {
            Err(Error::MalFormat(
                "Posts should be a array, maybe api has changed".into(),
            ))
        }
    }

    pub async fn fetch_pic(&self, url: impl IntoUrl) -> Result<Bytes> {
        debug!("fetch pic, url: {}", url.as_str());
        let res = self._fetch(url, &self.pic_client).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        Ok(res_bytes)
    }

    pub async fn fetch_emoticon(&self) -> Result<HashMap<String, String>> {
        let url = STATUSES_CONFIG_API;
        debug!("fetch emoticon, url: {url}");
        let res = self._fetch(url, &self.web_client).await?;
        let mut json: Value = res.json().await?;
        if json["ok"] != 1 {
            return Err(Error::ResourceGetFailed(format!(
                "fetched emoticon is not ok: {json:?}"
            )));
        }

        let mut res = HashMap::new();
        let Value::Object(emoticon) = json["data"]["emoticon"].take() else {
            return Err(Error::MalFormat(
                "the format of emoticon is unexpected".into(),
            ));
        };
        for (_, groups) in emoticon {
            let Value::Object(group) = groups else {
                return Err(Error::MalFormat(
                    "the format of emoticon is unexpected".into(),
                ));
            };
            for (_, emojis) in group {
                let Value::Array(emojis) = emojis else {
                    return Err(Error::MalFormat(
                        "the format of emoticon is unexpected".into(),
                    ));
                };
                for mut emoji in emojis {
                    let (Value::String(phrase), Value::String(url)) =
                        (emoji["phrase"].take(), emoji["url"].take())
                    else {
                        return Err(Error::MalFormat(
                            "the format of emoticon is unexpected".into(),
                        ));
                    };
                    res.insert(phrase, url);
                }
            }
        }
        Ok(res)
    }

    pub async fn fetch_fav_total_num(&self) -> Result<u32> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let ret_json: Value = self
            ._fetch(FAVORITES_TAGS_API, &self.web_client)
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

    pub async fn fetch_long_text_content(&self, mblogid: &str) -> Result<String> {
        let url = format!("{STATUSES_LONGTEXT_API}?id={mblogid}");
        debug!("fetch long text, url: {url}");
        let res = self._fetch(url, &self.web_client).await?;
        let long_text_meta = match res.json::<LongText>().await {
            Ok(res) => res,
            Err(e) if e.is_decode() => {
                return Err(Error::ResourceGetFailed("bypass weibo's bug".into()))
            }
            Err(e) => return Err(e.into()),
        };
        long_text_meta.get_content()
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
