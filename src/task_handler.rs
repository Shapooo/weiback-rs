use std::ops::RangeInclusive;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use log::{debug, error, info};
use tokio::time::sleep;

use crate::error::{Error, Result};
use crate::exporter::Exporter;
use crate::login::LoginInfo;
use crate::message::TaskStatus;
use crate::persister::Persister;
use crate::post_processor::PostProcessor;
use crate::web_fetcher::WebFetcher;

const SAVING_PERIOD: usize = 200;

static mut POSTS_TOTAL: u64 = 0;

#[derive(Debug)]
pub struct TaskHandler {
    exporter: Exporter,
    processer: PostProcessor,
    task_status: Arc<RwLock<TaskStatus>>,
    uid: &'static str,
}

impl TaskHandler {
    pub fn new(mut login_info: LoginInfo, task_status: Arc<RwLock<TaskStatus>>) -> Result<Self> {
        let uid = if let serde_json::Value::String(uid) = login_info["uid"].take() {
            Box::leak(Box::new(uid))
        } else {
            return Err(Error::MalFormat(format!(
                "no uid field in login_info: {:?}",
                login_info
            )));
        };
        let fetcher =
            WebFetcher::from_cookies(uid, serde_json::from_value(login_info["cookies"].take())?)?;
        let persister = Persister::new();

        Ok(TaskHandler {
            exporter: Exporter::new(),
            processer: PostProcessor::new(fetcher, persister),
            task_status,
            uid: Box::leak(Box::new(uid.to_string())),
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        self.processer.init().await?;
        let (web_total, db_total) = tokio::join!(
            self.processer.get_web_total_num(),
            self.processer.get_db_total_num()
        );
        let web_total = web_total?;
        unsafe {
            POSTS_TOTAL = web_total;
        }
        *self.task_status.write().unwrap() = TaskStatus::Init(web_total, db_total?);
        Ok(())
    }

    pub async fn unfavorite_posts(&self, range: RangeInclusive<u32>) {
        self.handle_task_res(self._unfavorite_posts(range).await)
            .await
    }

    async fn _unfavorite_posts(&self, range: RangeInclusive<u32>) -> Result<()> {
        let ids = self.processer.get_fav_ids_to_unfavorite(range).await?;
        let len = ids.len();
        for (i, id) in ids.into_iter().enumerate() {
            self.processer.unfavorite_post(id).await?;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let _ = self.task_status.try_write().map(|mut op| {
                let progress = i as f32 / len as f32;
                *op = TaskStatus::InProgress(progress, format!("已处理{i}条，共{len}条..."))
            });
        }

        Ok(())
    }

    pub async fn export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    ) {
        info!("fetch posts from local and export");
        self.handle_task_res(
            self._export_from_local(range, reverse, image_definition)
                .await,
        )
        .await;
    }

    async fn _export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    ) -> Result<()> {
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%H-%M"));
        let target_dir = std::env::current_dir()?.join(task_name);

        let mut local_posts = self
            .processer
            .load_fav_posts_from_db(range, reverse)
            .await?;
        let posts_sum = local_posts.len();
        debug!("fetched {} posts from local", posts_sum);

        let mut index = 1;
        loop {
            let subtask_name = format!("weiback-{index}");
            if local_posts.len() < SAVING_PERIOD {
                let html = self
                    .processer
                    .generate_html(local_posts, &subtask_name, image_definition)
                    .await?;
                self.exporter
                    .export_page(&subtask_name, html, &target_dir)
                    .await?;
                break;
            } else {
                let html = self
                    .processer
                    .generate_html(
                        local_posts.split_off(SAVING_PERIOD),
                        &subtask_name,
                        image_definition,
                    )
                    .await?;
                self.exporter
                    .export_page(&subtask_name, html, &target_dir)
                    .await?;
            }
            let _ = self.task_status.try_write().map(|mut op| {
                let progress = (posts_sum - local_posts.len()) as f32 / posts_sum as f32;
                *op = TaskStatus::InProgress(
                    progress,
                    format!(
                        "已处理{}条，共{}条...\n可能需要下载图片",
                        posts_sum - local_posts.len(),
                        posts_sum
                    ),
                )
            });
            index += 1;
        }
        Ok(())
    }

    pub async fn download_posts(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    ) {
        self.handle_task_res(
            self._download_posts(range, with_pic, image_definition)
                .await,
        )
        .await;
    }

    async fn _download_posts(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<()> {
        assert!(range.start() != &0);
        info!("pages download range is {range:?}");
        let mut total_downloaded: usize = 0;
        let post_total = unsafe { POSTS_TOTAL };
        let task_quota = (post_total.min(*range.end() as u64 * 20)
            - post_total.min(*range.start() as u64 * 20)) as f32;

        for page in range {
            let posts_sum = self
                .processer
                .download_fav_posts(self.uid, page, with_pic, image_definition)
                .await?;
            total_downloaded += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }

            let _ = self.task_status.try_write().map(|mut pro| {
                *pro = TaskStatus::InProgress(
                    total_downloaded as f32 / task_quota,
                    format!("已下载第{page}页...耐心等待，先干点别的"),
                )
            });
            sleep(Duration::from_secs(5)).await;
        }
        info!("fetched {total_downloaded} posts in total");
        Ok(())
    }

    async fn handle_task_res(&self, result: Result<()>) {
        let mut db_total = 0;
        let mut web_total = 0;
        let result = self
            ._handle_task_res(result, &mut web_total, &mut db_total)
            .await;
        match result {
            Err(err) => {
                error!("{err}");
                *self.task_status.write().unwrap() = TaskStatus::Error(format!("错误：{err}"));
            }
            Ok(()) => {
                info!("task finished");
                *self.task_status.write().unwrap() = TaskStatus::Finished(web_total, db_total);
            }
        }
    }

    async fn _handle_task_res(
        &self,
        result: Result<()>,
        web_total: &mut u64,
        db_total: &mut u64,
    ) -> Result<()> {
        result?;
        let (web_total_res, db_total_res) = tokio::join!(
            self.processer.get_web_total_num(),
            self.processer.get_db_total_num()
        );
        *web_total = web_total_res?;
        *db_total = db_total_res?;
        Ok(())
    }
}
