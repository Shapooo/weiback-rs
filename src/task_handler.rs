use std::collections::HashSet;
use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::Result;
use chrono;
use log::{debug, info};
use tokio::time::sleep;

use crate::config::Config;
use crate::exporter::{Exporter, HTMLPage, Picture};
use crate::html_generator::HTMLGenerator;
use crate::persister::Persister;
use crate::post_processor::PostProcessor;
use crate::resource_manager::ResourceManager;
use crate::utils::pic_url_to_file;
use crate::web_fetcher::WebFetcher;

#[derive(Debug)]
pub struct TaskHandler {
    resource_manager: ResourceManager,
    generator: HTMLGenerator,
    exporter: Exporter,
    config: Config,
    processer: PostProcessor,
}

impl TaskHandler {
    pub async fn build(config: Config) -> Result<Self> {
        let fetcher = WebFetcher::build(
            config.web_cookie.clone(),
            (!config.mobile_cookie.is_empty()).then_some(config.mobile_cookie.clone()),
        );
        let persister = Persister::build(&config.db).await?;
        let resource_manager = ResourceManager::build(fetcher, persister);
        let emoticon = resource_manager.get_emoticon().await?;
        Ok(TaskHandler {
            resource_manager,
            generator: HTMLGenerator::new(),
            exporter: Exporter::new(),
            config,
            processer: PostProcessor::new(emoticon),
        })
    }

    pub async fn download(&self, range: RangeInclusive<u64>) -> Result<()> {
        self.download_posts(range, false, false).await
    }

    pub async fn download_with_pic(&self, range: RangeInclusive<u64>) -> Result<()> {
        self.download_posts(range, true, false).await
    }

    pub async fn download_with_pic_and_export(&self, range: RangeInclusive<u64>) -> Result<()> {
        self.download_posts(range, true, true).await
    }

    async fn download_posts(
        &self,
        mut range: RangeInclusive<u64>,
        with_pic: bool,
        export: bool,
    ) -> Result<()> {
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%R"));

        if range.start() == &0 {
            range = RangeInclusive::new(1, *range.end());
        }
        info!("pages download range is {range:?}");
        let mut total_posts_sum = 0;
        let mut pic_to_fetch: HashSet<String> = HashSet::new();
        let mut html = String::new();
        for page in range {
            let posts_meta = self
                .resource_manager
                .get_fav_posts_from_web(self.config.uid.as_str(), page)
                .await?;
            let posts_sum = posts_meta.len();
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }

            if with_pic {
                for mut post in posts_meta {
                    self.processer.process_post(&mut post, &mut pic_to_fetch)?;
                    if export {
                        html.push_str(self.generator.generate_post(post).await?.as_str());
                    }
                }
            }
            sleep(Duration::from_secs(5)).await;
        }

        if export {
            let mut pics = Vec::new();
            for pic_url in pic_to_fetch {
                let name = pic_url_to_file(&pic_url).into();
                let blob = self.resource_manager.get_pic(&pic_url).await?;
                pics.push(Picture { name, blob });
            }
            let html = self.generator.generate_page(&html).await?;
            let page = HTMLPage { html, pics };
            self.exporter
                .export_page(task_name, page, std::path::PathBuf::from("./").as_path())
                .await?;
        }
        info!("fetched {total_posts_sum} posts in total");
        Ok(())
    }
}
