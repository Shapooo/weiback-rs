use log::info;
use std::future::Future;
use tokio::{self, sync::mpsc::Sender, time::sleep};
use weibosdk_rs::WeiboAPI;

use crate::config::get_config;
use crate::core::task::{BFOptions, BUOptions};
use crate::error::{Error, Result};
use crate::exporter::{ExportOptions, Exporter};
use crate::media_downloader::MediaDownloader;
use crate::message::{Message, TaskProgress, TaskType};
use crate::models::Post;
use crate::processing::PostProcesser;
use crate::storage::Storage;

#[derive(Debug, Clone)]
pub struct TaskHandler<W: WeiboAPI, S: Storage, E: Exporter, D: MediaDownloader> {
    api_client: W,
    storage: S,
    exporter: E,
    processer: PostProcesser<W, S, D>,
    msg_sender: Sender<Message>,
}

impl<W: WeiboAPI, S: Storage, E: Exporter, D: MediaDownloader> TaskHandler<W, S, E, D> {
    pub fn new(
        api_client: W,
        storage: S,
        exporter: E,
        downloader: D,
        msg_sender: Sender<Message>,
    ) -> Result<Self> {
        let processer = PostProcesser::new(
            api_client.clone(),
            storage.clone(),
            downloader,
            msg_sender.clone(),
        )?;
        Ok(TaskHandler {
            api_client,
            storage,
            exporter,
            processer,
            msg_sender,
        })
    }

    pub fn msg_sender(&self) -> &Sender<Message> {
        &self.msg_sender
    }

    async fn backup_procedure<F, Fut>(
        &self,
        task_id: u64,
        range: (u32, u32),
        count: u32,
        task_type: TaskType,
        page_backup_fn: F,
    ) -> Result<()>
    where
        F: Fn(u32) -> Fut,
        Fut: Future<Output = Result<usize>>,
    {
        let task_interval = get_config()
            .read()
            .map_err(|e| Error::Other(e.to_string()))?
            .backup_task_interval;

        let (mut start, mut end) = range;
        let mut total_downloaded: usize = 0;
        start = start.div_ceil(count);
        end = end.div_ceil(count);
        self.msg_sender
            .send(Message::TaskProgress(TaskProgress {
                r#type: task_type.clone(),
                task_id,
                total_increment: (end - start + 1) as u64,
                progress_increment: 0,
            }))
            .await?;

        for page in start..=end {
            let posts_sum = page_backup_fn(page).await?;
            total_downloaded += posts_sum;
            info!("fetched {posts_sum} posts in {page}th page");

            self.msg_sender
                .send(Message::TaskProgress(TaskProgress {
                    r#type: task_type.clone(),
                    task_id,
                    total_increment: 0,
                    progress_increment: 1,
                }))
                .await?;
            if page != end {
                sleep(task_interval).await;
            }
        }
        info!("fetched {total_downloaded} posts in total");
        Ok(())
    }

    // backup user posts
    pub(super) async fn backup_user(&self, task_id: u64, options: BUOptions) -> Result<()> {
        let count = get_config()
            .read()
            .map_err(|e| Error::Other(e.to_string()))?
            .weibo_api_config
            .status_count as u32;
        let uid = options.uid;
        let range = options.range.into_inner();
        info!("download user {uid} posts, from {} to {}", range.0, range.1);

        self.backup_procedure(task_id, range, count, TaskType::BackUser, |page| {
            self.backup_one_page(task_id, uid, page)
        })
        .await
    }

    // backup one page of posts of the user
    async fn backup_one_page(&self, task_id: u64, uid: i64, page: u32) -> Result<usize> {
        let posts = self.api_client.profile_statuses(uid, page).await?;
        let result = posts.len();
        self.processer.process(task_id, posts).await?;
        Ok(result)
    }

    // export favorite posts from weibo
    pub(super) async fn backup_favorites(&self, task_id: u64, options: BFOptions) -> Result<()> {
        let count = get_config()
            .read()
            .map_err(|e| Error::Other(e.to_string()))?
            .weibo_api_config
            .fav_count as u32;
        let range = options.range.into_inner();
        info!("favorites download from {} to {}", range.0, range.1);

        self.backup_procedure(task_id, range, count, TaskType::BackFav, |page| {
            self.backup_one_fav_page(task_id, page)
        })
        .await
    }

    // backup one page of favorites
    async fn backup_one_fav_page(&self, task_id: u64, page: u32) -> Result<usize> {
        let posts = self.api_client.favorites(page).await?;
        let result = posts.len();
        let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
        self.processer.process(task_id, posts).await?;

        // call mark_user_backed_up after all posts inserted, to ensure the post is in db
        for id in ids {
            self.storage.mark_post_favorited(id).await?;
        }
        Ok(result)
    }

    // unfavorite all posts that are in weibo favorites
    pub(super) async fn unfavorite_posts(&self, task_id: u64) -> Result<()> {
        let task_interval = get_config()
            .read()
            .map_err(|e| Error::Other(e.to_string()))?
            .other_task_interval;
        let ids = self.storage.get_posts_id_to_unfavorite().await?;
        let len = ids.len();
        self.msg_sender
            .send(Message::TaskProgress(TaskProgress {
                r#type: TaskType::Unfav,
                task_id,
                total_increment: 0,
                progress_increment: len as u64,
            }))
            .await?;
        for id in ids {
            self.api_client.favorites_destroy(id).await?;
            info!("post {id} unfavorited");
            self.msg_sender
                .send(Message::TaskProgress(TaskProgress {
                    r#type: TaskType::Unfav,
                    task_id,
                    total_increment: 0,
                    progress_increment: 1,
                }))
                .await?;
            tokio::time::sleep(task_interval).await;
        }
        Ok(())
    }

    pub async fn export_from_local(&self, mut options: ExportOptions) -> Result<()> {
        let posts_sum = self.get_favorited_sum().await?;
        info!("fetched {posts_sum} posts from local");
        let (mut start, end) = options.range.into_inner();
        let task_name = options.export_task_name.to_owned();
        let limit = options.posts_per_html;
        for index in 1.. {
            options.range = start..=end.min(start + limit);
            let local_posts = self.load_fav_posts_from_db(&options).await?;
            if local_posts.is_empty() {
                break;
            }

            let subtask_name = format!("{task_name}_{index}");
            options.export_task_name = subtask_name;
            let html = self.processer.generate_html(local_posts, &options).await?;
            self.exporter.export_page(html, &options).await?;

            if start == end {
                break;
            }
            start = end.min(start + limit);
        }
        Ok(())
    }

    pub async fn load_fav_posts_from_db(&self, options: &ExportOptions) -> Result<Vec<Post>> {
        self.storage.get_posts(options).await
    }

    // get total number of favorites in local database
    pub async fn get_favorited_sum(&self) -> Result<u32> {
        self.storage.get_favorited_sum().await
    }
}
