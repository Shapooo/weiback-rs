use std::ops::RangeInclusive;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono;
use log::{debug, error, info};
use tokio::time::sleep;

use crate::config::Config;
use crate::data::Posts;
use crate::error::Result;
use crate::exporter::Exporter;
use crate::message::TaskStatus;
use crate::persister::Persister;
use crate::post_processor::PostProcessor;
use crate::resource_manager::ResourceManager;
use crate::web_fetcher::WebFetcher;

const SAVING_PERIOD: usize = 200;

#[derive(Debug)]
pub struct TaskHandler {
    exporter: Exporter,
    config: Config,
    processer: PostProcessor,
    task_status: Arc<RwLock<TaskStatus>>,
}

impl TaskHandler {
    pub fn new(config: Config, task_status: Arc<RwLock<TaskStatus>>) -> Result<Self> {
        let fetcher = WebFetcher::new(
            config.web_cookie.clone(),
            (!config.mobile_cookie.is_empty()).then_some(config.mobile_cookie.clone()),
        );
        let persister = Persister::new(&config.db)?;
        let resource_manager = ResourceManager::new(fetcher, persister);
        Ok(TaskHandler {
            exporter: Exporter::new(),
            config,
            processer: PostProcessor::new(resource_manager),
            task_status,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        self.processer.init().await?;
        let (web_total, db_total) = tokio::join!(
            self.processer.get_web_total_num(),
            self.processer.get_db_total_num()
        );
        *self.task_status.write().unwrap() =
            TaskStatus::Info(format!("{}  {}", web_total?, db_total?));
        Ok(())
    }

    pub async fn download_meta_only(&self, range: RangeInclusive<u32>) {
        info!("downloading posts meta data...");
        self._download_posts(range, false, false).await;
    }

    pub async fn download_with_pic(&self, range: RangeInclusive<u32>) {
        info!("download posts with pics...");
        self._download_posts(range, true, false).await;
    }

    pub async fn export_from_net(&self, range: RangeInclusive<u32>) {
        info!("download posts with pic, and export...");
        self._download_posts(range, true, true).await;
    }

    pub async fn export_from_local(&self, range: RangeInclusive<u32>, reverse: bool) {
        info!("fetch posts from local and export");
        match self._export_from_local(range, reverse).await {
            Err(err) => {
                error!("{err}");
                *self.task_status.write().unwrap() = TaskStatus::Error(format!("错误：{err}"));
            }
            _ => {}
        }
    }

    async fn _export_from_local(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<()> {
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%H-%M"));
        let target_dir = std::env::current_dir()?.join(task_name);

        let mut post_acc = Vec::new();
        let local_posts = self.processer.get_fav_post_from_db(range, reverse).await?;
        let posts_sum = local_posts.len();
        debug!("fetched {} posts from local", posts_sum);
        for (i, post) in local_posts.into_iter().enumerate() {
            post_acc.push(post);
            if i % SAVING_PERIOD == SAVING_PERIOD - 1 || i == posts_sum - 1 {
                let subtask_name = format!("weiback-{}", (i + SAVING_PERIOD - 1) / SAVING_PERIOD);
                let html = self
                    .processer
                    .generate_html(Posts { data: post_acc }, &subtask_name)
                    .await?;
                post_acc = Vec::new();

                self.exporter
                    .export_page(&subtask_name, html, &target_dir)
                    .await?;
            }
        }
        *self.task_status.write().unwrap() = TaskStatus::Finished;
        Ok(())
    }

    async fn _download_posts(&self, range: RangeInclusive<u32>, with_pic: bool, export: bool) {
        match self.__download_posts(range, with_pic, export).await {
            Err(err) => {
                error!("{err}");
                *self.task_status.write().unwrap() = TaskStatus::Error(format!("错误：{err}"));
            }
            _ => {}
        }
    }

    async fn __download_posts(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        export: bool,
    ) -> Result<()> {
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%H-%M"));
        let target_dir = std::env::current_dir().unwrap().join(task_name);

        assert!(range.start() != &0);
        info!("pages download range is {range:?}");
        let mut total_posts_sum = 0;
        let mut posts_acc = Posts::new();
        let end = *range.end();
        for (i, page) in range.enumerate() {
            let posts = self
                .processer
                .get_fav_posts_from_web(self.config.uid.as_str(), page)
                .await?;
            let posts_sum = posts.len();
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }

            if with_pic && !export {
                self.processer.save_post_pictures(posts).await?
            } else if export {
                posts_acc.append(posts);
                if i % SAVING_PERIOD == SAVING_PERIOD - 1 || posts_sum == 0 {
                    let subtask_name = format!("weiback-{}", page);
                    self.exporter
                        .export_page(
                            &subtask_name,
                            self.processer
                                .generate_html(posts_acc, &subtask_name)
                                .await?,
                            &target_dir,
                        )
                        .await?;
                    posts_acc = Posts::new();
                }
            }
            let _ = self
                .task_status
                .try_write()
                .map(|mut pro| *pro = TaskStatus::InProgress(i as f32 / end as f32, "".into()));
            sleep(Duration::from_secs(5)).await;
        }
        info!("fetched {total_posts_sum} posts in total");
        *self.task_status.write().unwrap() = TaskStatus::Finished;
        Ok(())
    }
}
