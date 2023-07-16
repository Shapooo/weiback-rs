use std::ops::RangeInclusive;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono;
use log::{debug, error, info};
use tokio::time::sleep;

use crate::error::Result;
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
        let fetcher =
            WebFetcher::from_cookies(serde_json::from_value(login_info["cookies"].take())?)?;
        let persister = Persister::new();
        let uid = if let serde_json::Value::String(uid) = login_info["uid"].take() {
            Box::leak(Box::new(uid))
        } else {
            ""
        };
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
        *self.task_status.write().unwrap() = TaskStatus::Info(format!(
            "账号共 {} 条收藏\n本地保存有 {} 条收藏",
            web_total, db_total?
        ));
        Ok(())
    }

    pub async fn export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    ) {
        info!("fetch posts from local and export");
        match self
            ._export_from_local(range, reverse, image_definition)
            .await
        {
            Err(err) => {
                error!("{err}");
                *self.task_status.write().unwrap() = TaskStatus::Error(format!("错误：{err}"));
            }
            _ => {}
        }
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
                *op = TaskStatus::InProgress(
                    local_posts.len() as f32 / posts_sum as f32,
                    "导出中...可能需要下载图片".into(),
                )
            });
            index += 1;
        }
        *self.task_status.write().unwrap() = TaskStatus::Finished;
        Ok(())
    }

    pub async fn download_posts(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    ) {
        match self
            ._download_posts(range, with_pic, image_definition)
            .await
        {
            Err(err) => {
                error!("{err}");
                *self.task_status.write().unwrap() = TaskStatus::Error(format!("错误：{err}"));
            }
            _ => {}
        }
    }

    async fn _download_posts(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<()> {
        assert!(range.start() != &0);
        info!("pages download range is {range:?}");
        let mut total_posts_sum: usize = 0;
        let end = if *range.end() == u32::MAX {
            (unsafe { POSTS_TOTAL }) as f32
        } else {
            unsafe { POSTS_TOTAL }.min(*range.end() as u64 * 20) as f32
        };

        for (i, page) in range.enumerate() {
            let posts_sum = self
                .processer
                .download_fav_posts(self.uid, page, with_pic, image_definition)
                .await?;
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }

            let _ = self.task_status.try_write().map(|mut pro| {
                *pro =
                    TaskStatus::InProgress(i as f32 / end, "下载中...耐心等待，先干点别的".into())
            });
            sleep(Duration::from_secs(5)).await;
        }
        info!("fetched {total_posts_sum} posts in total");
        *self.task_status.write().unwrap() = TaskStatus::Finished;
        Ok(())
    }
}
