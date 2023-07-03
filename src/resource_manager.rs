use std::collections::HashMap;

use bytes::Bytes;
use futures::future::join_all;
use log::info;

use crate::error::{Error, Result};
use crate::{data::Posts, persister::Persister, web_fetcher::WebFetcher};

#[derive(Debug)]
pub struct ResourceManager {
    web_fetcher: WebFetcher,
    persister: Persister,
}

impl ResourceManager {
    pub fn new(web_fetcher: WebFetcher, persister: Persister) -> Self {
        Self {
            web_fetcher,
            persister,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        self.persister.init().await?;
        Ok(())
    }

    pub async fn get_pic(&self, url: &str) -> Result<Bytes> {
        let url = crate::utils::strip_url_queries(url);
        let res = self.persister.query_img(url).await;
        if let Err(Error::NotInLocal) = res {
            let pic = self.web_fetcher.fetch_pic(url).await?;
            self.persister.insert_img(url, &pic).await?;
            Ok(pic)
        } else {
            Ok(res?)
        }
    }

    pub async fn get_fav_posts_from_web(&self, uid: &str, page: u32) -> Result<Posts> {
        let data = self.web_fetcher.fetch_posts_meta(uid, page).await?.data;
        let data: Vec<serde_json::Value> = join_all(data.into_iter().map(|post| async {
            self.persister.insert_post(&post).await?;
            Ok(post)
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<serde_json::Value>>>()?;

        Ok(Posts { data })
    }

    pub async fn get_web_total_num(&self) -> Result<u64> {
        self.web_fetcher.fetch_fav_total_num().await
    }

    pub async fn get_db_total_num(&self) -> Result<u64> {
        Ok(self.persister.query_db_total_num().await?)
    }

    pub async fn get_fav_post_from_db(
        &self,
        range: std::ops::RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Posts> {
        info!("get {:?} post (reverse? {}) from db", range, reverse);
        let limit = (range.end() - range.start()) + 1;
        let offset = *range.start() - 1;
        Ok(self.persister.query_posts(limit, offset, reverse).await?)
    }

    pub async fn get_emoticon(&self) -> Result<HashMap<String, String>> {
        Ok(self.web_fetcher.fetch_emoticon().await?)
    }
}
