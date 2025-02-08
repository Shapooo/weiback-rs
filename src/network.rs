mod http_client;
mod post_internal;
mod user_internal;

use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{debug, error, info, trace, warn};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, IntoUrl, Response, StatusCode,
};
use reqwest_cookie_store::CookieStoreMutex;
use serde_json::{from_value, Value};

use crate::app::{
    models::{LongText, Post, User},
    ports::Network,
};
use http_client::HttpClient;

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
    http_client: HttpClient,
    auth_status: AuthStatus,
}

impl NetworkImpl {
    pub fn new(cookie_store: Arc<CookieStoreMutex>) -> Result<Self> {
        let http_client = HttpClient::new(cookie_store)?;
        let post_client = PostClient::new(http_client.clone());

        let network = NetworkImpl {
            uid: None,
            http_client: http_client,
            auth_status: AuthStatus::UnsignedIn,
        };

        Ok(network)
    }
}

impl Network for NetworkImpl {
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
        self._get_posts(url).await
    }

    async fn get_favorate_posts(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        let url = Post::get_favorite_download_url(uid, page);
        debug!("fetch fav meta page, url: {url}");
        self._get_posts(url).await
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

    async fn get_long_text(&self, mblogid: &str) -> Result<Option<LongText>> {
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
        Ok(Some(long_text_meta))
    }
}
