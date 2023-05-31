use std::time::Duration;

use crate::config::Config;
use crate::fetcher::Fetcher;
use crate::persister::Persister;
use anyhow::Result;
use log::{debug, info};
use tokio::time::sleep;

#[derive(Debug)]
pub struct TaskHandler {
    fetcher: Fetcher,
    persister: Persister,
    config: Config,
}

impl TaskHandler {
    pub fn build(config: Config) -> Result<Self> {
        let fetcher = Fetcher::new(config.web_cookie.clone(), None);
        let persister = Persister::build(config.db.clone())?;
        Ok(TaskHandler {
            fetcher,
            persister,
            config,
        })
    }

    pub async fn fetch_all_page(&self) -> Result<()> {
        let fav_total_num = self.fetcher.get_fav_total_num().await?;
        info!("there are {fav_total_num} fav posts in total");
        let mut page = 1;
        let mut total_posts_sum = 0;
        loop {
            let posts = self
                .fetcher
                .fetch_posts_meta(self.config.uid.as_str(), page)
                .await?;
            let posts_sum = posts.len();
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }
            posts
                .iter()
                .for_each(|post| self.persister.insert_post(post).unwrap());
            page += 1;
            sleep(Duration::from_secs(5)).await;
        }
        info!("fetched {total_posts_sum} posts in total");
        Ok(())
    }
}
