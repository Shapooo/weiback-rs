mod http_client;
mod picture_internal;
mod post_internal;
mod user_internal;

use std::sync::Arc;

use anyhow::{anyhow, Ok, Result};
use log::{debug, trace, warn};
use reqwest_cookie_store::CookieStoreMutex;
use serde_json::Value;

use super::{
    app::search_args::SearchArgs,
    models::{LongText, Post},
    ports::Network,
};
use http_client::HttpClient;
use post_internal::PostClient;

const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";

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
    post_client: PostClient,
}

impl NetworkImpl {
    pub fn new(cookie_store: Arc<CookieStoreMutex>) -> Result<Self> {
        let http_client = HttpClient::new(cookie_store)?;
        let post_client = PostClient::new(http_client.clone());

        let network = NetworkImpl {
            uid: None,
            http_client: http_client,
            auth_status: AuthStatus::UnsignedIn,
            post_client: post_client,
        };

        Ok(network)
    }
}

impl Network for NetworkImpl {
    async fn get_favorite_num(&self) -> Result<u32> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let ret_json: Value = self
            .http_client
            .get(FAVORITES_TAGS_API)
            .await?
            .json()
            .await?;
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

    async fn get_posts(&self, uid: i64, page: u32, search_args: &SearchArgs) -> Result<Vec<Post>> {
        self.post_client.get_posts(uid, page, search_args).await
    }

    async fn get_favorite_posts(&self, uid: i64, page: u32) -> Result<Vec<Post>> {
        self.post_client.get_favorite_posts(uid, page).await
    }

    async fn unfavorite_post(&self, id: i64) -> Result<()> {
        self.post_client.unfavorite_post(id).await
    }

    async fn get_mobile_post(&self, mblogid: &str) -> Result<Post> {
        self.post_client.get_mobile_post(mblogid).await
    }

    async fn get_long_text(&self, mblogid: &str) -> Result<Option<LongText>> {
        self.post_client.get_long_text(mblogid).await
    }
}
