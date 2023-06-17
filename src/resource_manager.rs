use std::collections::HashMap;

use anyhow;
use bytes::Bytes;
use serde_json::Value;

use crate::{data::Posts, persister::Persister, web_fetcher::WebFetcher};

#[derive(Debug)]
pub struct ResourceManager {
    web_fetcher: WebFetcher,
    persister: Persister,
}

impl ResourceManager {
    pub fn build(web_fetcher: WebFetcher, persister: Persister) -> Self {
        Self {
            web_fetcher,
            persister,
        }
    }

    pub async fn get_pic(&self, url: &str) -> anyhow::Result<Bytes> {
        let res = self.persister.query_img(url).await;
        if let Err(sqlx::error::Error::RowNotFound) = res {
            let pic = self.web_fetcher.fetch_pic(url).await?;
            self.persister.insert_img(url, &pic).await?;
            Ok(pic)
        } else {
            Ok(res?)
        }
    }

    pub async fn get_fav_posts_from_web(&self, uid: &str, page: u64) -> anyhow::Result<Posts> {
        let posts = self.web_fetcher.fetch_posts_meta(uid, page).await?.data;
        let mut res = Vec::new();
        for mut post in posts {
            let is_long_text = &post["isLongText"];
            if is_long_text.is_boolean() && is_long_text.as_bool().unwrap() {
                let mblogid = &post["mblogid"];
                let long_text = self
                    .web_fetcher
                    .fetch_long_text_content(mblogid.as_str().unwrap())
                    .await?;
                post["text_raw"] = Value::String(long_text);
            }
            self.persister.insert_post(&post).await?;
            res.push(post);
        }

        Ok(Posts { data: res })
    }

    pub async fn get_fav_total_num(&self) -> anyhow::Result<u64> {
        self.web_fetcher.fetch_fav_total_num().await
    }

    pub async fn get_fav_post_from_db(
        &self,
        _range: std::ops::RangeInclusive<u64>,
    ) -> anyhow::Result<Posts> {
        todo!()
    }

    pub async fn get_emoticon(&self) -> anyhow::Result<HashMap<String, String>> {
        let res = self.web_fetcher.fetch_emoticon().await?;
        todo!()
    }
}
