use log::{debug, error, info};
use std::future::Future;
use tokio::{self, sync::mpsc::Sender, time::sleep};
use weibosdk_rs::WeiboAPI;

use crate::config::get_config;
use crate::core::task::{BFOptions, BUOptions, ExportOptions};
use crate::error::Result;
use crate::exporter::Exporter;
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
        info!("Starting backup procedure for task {task_id:?}, type: {task_type:?}");
        let task_interval = get_config().read()?.backup_task_interval;

        let (mut start, mut end) = range;
        let mut total_downloaded: usize = 0;
        start = start.div_ceil(count);
        end = end.div_ceil(count);
        debug!("Backup task {task_id} page range: {start}..={end}");
        self.msg_sender
            .send(Message::TaskProgress(TaskProgress {
                r#type: task_type.clone(),
                task_id,
                total_increment: (end - start + 1) as u64,
                progress_increment: 0,
            }))
            .await?;

        for page in start..=end {
            let posts_sum = page_backup_fn(page).await.map_err(|e| {
                error!("Failed to backup page {page} for task {task_id}: {e}");
                e
            })?;

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
        info!(
            "Backup procedure for task {task_id:?} finished. Fetched {total_downloaded} posts in total"
        );
        Ok(())
    }

    // backup user posts
    pub(super) async fn backup_user(&self, task_id: u64, options: BUOptions) -> Result<()> {
        let count = get_config().read()?.weibo_api_config.status_count as u32;
        let uid = options.uid;
        let range = options.range.into_inner();
        info!(
            "Start backing up user {uid} posts, from {} to {}",
            range.0, range.1
        );

        self.backup_procedure(task_id, range, count, TaskType::BackUser, |page| {
            self.backup_one_page(task_id, uid, page)
        })
        .await?;

        info!("Finished backing up user {uid} posts.");
        Ok(())
    }

    // backup one page of posts of the user
    async fn backup_one_page(&self, task_id: u64, uid: i64, page: u32) -> Result<usize> {
        debug!("Backing up page {page} for user {uid}, task {task_id}");
        let posts = self.api_client.profile_statuses(uid, page).await?;
        let result = posts.len();
        self.processer.process(task_id, posts).await?;
        Ok(result)
    }

    // export favorite posts from weibo
    pub(super) async fn backup_favorites(&self, task_id: u64, options: BFOptions) -> Result<()> {
        let count = get_config().read()?.weibo_api_config.fav_count as u32;
        let range = options.range.into_inner();
        info!("Start backing up favorites from {} to {}", range.0, range.1);

        self.backup_procedure(task_id, range, count, TaskType::BackFav, |page| {
            self.backup_one_fav_page(task_id, page)
        })
        .await?;
        info!("Finished backing up favorites.");
        Ok(())
    }

    // backup one page of favorites
    async fn backup_one_fav_page(&self, task_id: u64, page: u32) -> Result<usize> {
        debug!("Backing up favorites page {page}, task {task_id}");
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
        info!("Starting unfavorite posts task {task_id}");
        let task_interval = get_config().read()?.other_task_interval;
        let ids = self.storage.get_posts_id_to_unfavorite().await?;
        let len = ids.len();
        info!("Found {len} posts to unfavorite");
        self.msg_sender
            .send(Message::TaskProgress(TaskProgress {
                r#type: TaskType::Unfav,
                task_id,
                total_increment: len as u64,
                progress_increment: 0,
            }))
            .await?;
        for (i, id) in ids.into_iter().enumerate() {
            if let Err(e) = self.api_client.favorites_destroy(id).await {
                error!("Failed to unfavorite post {id}: {e}");
                continue;
            }
            info!("Post {id} unfavorited successfully");
            self.msg_sender
                .send(Message::TaskProgress(TaskProgress {
                    r#type: TaskType::Unfav,
                    task_id,
                    total_increment: 0,
                    progress_increment: 1,
                }))
                .await?;
            if i < len - 1 {
                tokio::time::sleep(task_interval).await;
            }
        }
        info!("Unfavorite posts task {task_id} finished");
        Ok(())
    }

    pub async fn export_from_local(&self, mut options: ExportOptions) -> Result<()> {
        info!("Starting export from local with options: {options:?}");
        let limit = get_config().read()?.posts_per_html;
        let posts_sum = self.get_favorited_sum().await?;
        info!("Found {posts_sum} favorited posts in local database");
        let (mut start, end) = options.range.into_inner();
        for index in 1.. {
            options.range = start..=end.min(start + limit);
            debug!("Exporting range: {:?}", options.range);
            let local_posts = self.load_favorites(&options).await?;
            if local_posts.is_empty() {
                info!("No more posts to export. Exiting loop.");
                break;
            }

            let page_name = format!("{}_{}", options.task_name, index);
            let html = self
                .processer
                .generate_html(local_posts, &page_name)
                .await?;
            self.exporter
                .export_page(html, &page_name, &options.export_dir)
                .await?;

            if start >= end {
                break;
            }
            start = end.min(start + limit);
        }
        info!("Finished exporting from local");
        Ok(())
    }

    pub async fn load_favorites(&self, options: &ExportOptions) -> Result<Vec<Post>> {
        self.storage
            .get_favorites(options.range.clone(), options.reverse)
            .await
    }

    // get total number of favorites in local database
    pub async fn get_favorited_sum(&self) -> Result<u32> {
        self.storage.get_favorited_sum().await
    }
}
