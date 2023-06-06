use std::time::Duration;

use crate::config::Config;
use crate::exporter::Exporter;
use crate::fetcher::Fetcher;
use crate::generator::{HTMLGenerator, HTMLPosts};
use crate::persister::Persister;

use anyhow::Result;
use chrono;
use log::{debug, info};
use tokio::time::sleep;

#[derive(Debug)]
pub struct TaskHandler {
    fetcher: Fetcher,
    persister: Persister,
    generator: HTMLGenerator,
    exporter: Exporter,
    config: Config,
}

impl TaskHandler {
    pub async fn build(config: Config) -> Result<Self> {
        let fetcher = Fetcher::build(
            config.web_cookie.clone(),
            if !config.mobile_cookie.is_empty() {
                Some(config.mobile_cookie.clone())
            } else {
                None
            },
        );
        let persister = Persister::build(&config.db).await?;
        Ok(TaskHandler {
            fetcher,
            persister,
            generator: HTMLGenerator::new(),
            exporter: Exporter::new(),
            config,
        })
    }

    pub async fn fetch_all_page(&self) -> Result<()> {
        let fav_total_num = self.fetcher.fetch_fav_total_num().await?;
        info!("there are {fav_total_num} fav posts in total");
        let mut total_posts_sum = 0;
        let mut posts = HTMLPosts::new();
        for page in 1.. {
            let posts_meta = self
                .fetcher
                .fetch_posts_meta(self.config.uid.as_str(), page)
                .await?;
            let posts_sum = posts_meta.len();
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }
            for post in posts_meta.into_iter() {
                posts.merge(self.generator.generate_post(post, &self.fetcher).await?);
            }
            sleep(Duration::from_secs(5)).await;
        }
        let html_page = self.generator.generate_page(posts).await?;
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%R"));
        self.exporter
            .export_page(
                task_name,
                html_page,
                std::path::PathBuf::from("./").as_path(),
            )
            .await?;
        info!("fetched {total_posts_sum} posts in total");
        Ok(())
    }
}
