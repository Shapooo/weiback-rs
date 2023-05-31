use std::time::Duration;

use crate::config::Config;
use crate::fetcher::Fetcher;
use crate::persister::Persister;
use anyhow::Result;
use log::{debug, info};
use tokio::time::sleep;

const POST_PER_PAGE: u64 = 20;

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
        let mut page_sum = self.fetcher.get_fav_page_sum().await?;
        page_sum = (page_sum + POST_PER_PAGE - 1) / POST_PER_PAGE;
        debug!("total page num is {}", page_sum);
        info!("start to fetch all page");
        // self.fetch_page((1, page_sum)).await
        self.fetch_page((1, 10)).await
    }

    async fn fetch_page(&self, range: (u64, u64)) -> Result<()> {
        for i in (range.0..=range.1).rev() {
            let posts = self
                .fetcher
                .fetch_posts_meta(self.config.uid.as_str(), i)
                .await?;
            posts
                .iter()
                .for_each(|post| self.persister.insert_post(post).unwrap());
            sleep(Duration::from_secs(5)).await;
        }
        Ok(())
    }
}
