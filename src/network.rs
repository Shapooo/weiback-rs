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
use crate::app::service::search_args::SearchArgs;
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

    async fn post(&self, url: impl IntoUrl, body: &Value) -> Result<Response> {
        let request = self.http_client.post(url).json(body).build()?;
        self._http_common(request).await
    }

    async fn get(&self, url: impl IntoUrl) -> Result<Response> {
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

        } else {
            Err(anyhow!(
                "fetch mobile post {} failed, with message {}",
                mblogid,
                res["message"]
            ))
        }
    }

    fn with_process_long_text(mut post: Post, long_text: String) -> Post {
        if post.is_long_text {
            post.text_raw = long_text;
        }
        post
    }

    async fn with_process_client_only(&self, mut post: Post) -> Result<Post> {
        if post.client_only {
            post = self.get_mobile_post(&post.mblogid).await?;
        }
        Ok(post)
    }

    async fn posts_process(&self, posts: Vec<Value>) -> Result<Vec<Post>> {
        let posts = posts
            .into_iter()
            .map(|post| post.try_into())
            .collect::<Result<Vec<Post>>>()?;
        debug!("get raw {} posts", posts.len());
        let posts = join_all(posts.into_iter().map(|post| async {
            let post = self.with_process_client_only(post).await?;
            // self.with_process_long_text(fetcher)
            // .await
            anyhow::Ok(post)
        }))
        .await
        .into_iter()
        .filter_map(|post| match post {
            // network errors usually recoverable, so just ignore it
            // TODO: collect failed post and retry
            Ok(post) => Some(post),
            Err(e) => {
                error!("process post failed: {}", e);
                None
            }
        })
        .collect::<Vec<_>>();
        Ok(posts)
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

    async fn get_posts(&self, uid: i64, page: u32, search_args: &SearchArgs) -> Result<Vec<Post>> {
        let url = Post::get_posts_download_url(uid, page, search_args);
        debug!("fetch meta page, url: {url}");
        let mut json: Value = self.get(url).await?.json().await?;
        trace!("get json: {json:?}");
        if json["ok"] != 1 {
            Err(anyhow!("fetched data is not ok: {json:?}"))
        } else if let Value::Array(posts) = json["data"]["list"].take() {
            Ok(self.posts_process(posts).await?)
        } else {
            Err(anyhow!("Posts should be a array, maybe api has changed"))
        }
    }

    async fn get_favorate_posts(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let url = Post::get_favorite_download_url(uid, page);
        debug!("fetch fav meta page, url: {url}");
        let mut posts: Value = self.get(url).await?.json().await?;
        trace!("get json: {posts:?}");
        if posts["ok"] != 1 {
            Err(anyhow!("fetched data is not ok: {posts:?}"))
        } else if let Value::Array(posts) = posts["data"].take() {
            Ok(self.posts_process(posts).await?)
        } else {
            Err(anyhow!("Posts should be a array, maybe api has changed"))
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

    async fn get_mobile_post(&self, mblogid: &str) -> Result<Post> {
        let url = Post::get_mobile_download_url(mblogid);
        info!("fetch client only post url: {}", &url);
        let mut res: Value = self.get(url).await?.json().await?;
        if res["ok"] == 1 {
            // let post = Self::convert_mobile2pc_post(res["data"].take())?;
            let post = res["data"].take().try_into()?;
            Ok(post)
        } else {
            Err(anyhow!(
                "fetch mobile post {} failed, with message {}",
                mblogid,
                res["message"]
            ))
        }
    }

    async fn get_long_text(&self, mblogid: &str) -> Result<Option<String>> {
        let url = LongText::get_long_text_url(mblogid);
        debug!("fetch long text, url: {url}");
        let res = self.get(url).await?;
        let long_text_meta = match res.json::<LongText>().await {
            Ok(res) => res,
            Err(e) if e.is_decode() => {
                // bypass post pictures folding
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        Ok(Some(long_text_meta.get_content()?))
    }
}
