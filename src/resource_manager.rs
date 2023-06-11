use anyhow;
use bytes::Bytes;

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
        todo!()
    }

    pub async fn get_posts(&self, uid: &str, page: u64) -> anyhow::Result<Posts> {
        todo!()
    }

    pub async fn get_fav_total_num(&self) -> anyhow::Result<u64> {
        todo!()
    }

    pub async fn get_long_text_content(&self, mblogid: &str) -> anyhow::Result<String> {
        todo!()
    }
}
