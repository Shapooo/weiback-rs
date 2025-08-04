use std::time::Duration;

use log::info;
use tokio::{self, sync::mpsc::Sender, time::sleep};
use weibosdk_rs::WeiboAPI;

use crate::core::task::{BFOptions, BUOptions};
use crate::error::Result;
use crate::exporter::{ExportOptions, Exporter};
use crate::media_downloader::MediaDownloader;
use crate::message::{Message, TaskProgress};
use crate::models::Post;
use crate::processing::PostProcesser;
use crate::storage::Storage;

const SAVING_PERIOD: usize = 200;
const BACKUP_TASK_INTERVAL: Duration = Duration::from_secs(3);
const OTHER_TASK_INTERVAL: Duration = Duration::from_secs(1);

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
        let processer = PostProcesser::new(api_client.clone(), storage.clone(), downloader)?;
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

    // backup one page of posts of the user
    async fn backup_one_page(&self, uid: i64, page: u32) -> Result<usize> {
        let posts = self.api_client.profile_statuses(uid, page).await?;
        let result = posts.len();
        for post in posts.iter() {
            self.storage.save_post(post).await?;
        }

        Ok(result)
    }

    // backup one page of favorites
    async fn backup_one_fav_page(&self, page: u32, options: BFOptions) -> Result<usize> {
        let posts = self.api_client.favorites(page).await?;
        let result = posts.len();
        let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
        self.processer.process(posts, &options).await?;

        // call mark_user_backed_up after all posts inserted, to ensure the post is in db
        for id in ids {
            self.storage.mark_post_favorited(id).await?;
        }

        Ok(result)
    }

    pub async fn load_fav_posts_from_db(&self, options: &ExportOptions) -> Result<Vec<Post>> {
        self.storage.get_posts(options).await
    }

    // get total number of favorites in local database
    pub async fn get_db_total_num(&self) -> Result<u32> {
        self.storage.get_favorited_sum().await
    }
    // unfavorite all posts that are in weibo favorites
    pub(super) async fn unfavorite_posts(&self) -> Result<()> {
        let ids = self.storage.get_posts_id_to_unfavorite().await?;
        let len = ids.len();
        let task_progress = TaskProgress {
            id: 0,
            total_increment: 0,
            current_increment: len as u64,
        };
        self.msg_sender
            .send(Message::TaskProgress(task_progress))
            .await?;
        for (mut i, id) in ids.into_iter().enumerate() {
            i = i + 1;
            self.api_client.favorites_destroy(id).await?;
            info!("post {id} unfavorited");
            let task_progress = TaskProgress {
                id: 0,
                total_increment: 0,
                current_increment: 1,
            };
            self.msg_sender
                .send(Message::TaskProgress(task_progress))
                .await?;
            tokio::time::sleep(OTHER_TASK_INTERVAL).await;
        }
        Ok(())
    }

    // backup user posts
    pub(super) async fn backup_user(&self, options: BUOptions) -> Result<()> {
        let uid = options.uid;
        info!("download user {uid} posts");

        let mut page = 1;
        loop {
            let len = self.backup_one_page(uid, page).await?;
            info!("fetched {len} posts in {page}th page");
            if len == 0 {
                break;
            }

            let task_progress = TaskProgress {
                id: 0,
                total_increment: len as u64,
                current_increment: len as u64,
            };
            self.msg_sender
                .send(Message::TaskProgress(task_progress))
                .await?;
            page += 1;
            sleep(BACKUP_TASK_INTERVAL).await;
        }
        Ok(())
    }

    pub async fn export_from_local(&self, mut options: ExportOptions) -> Result<()> {
        let posts_sum = self.get_db_total_num().await?;
        info!("fetched {} posts from local", posts_sum);
        // let target_dir = options.export_path.clone();
        let task_name = options.export_task_name.to_owned();
        let limit = options.posts_per_html;
        let task_progress = TaskProgress {
            id: 0,
            total_increment: posts_sum as u64,
            current_increment: 0,
        };
        self.msg_sender
            .send(Message::TaskProgress(task_progress))
            .await?;
        let mut offset = 0;
        let mut index = 1;
        loop {
            let mut opt = options.clone();
            opt.range = offset..=offset + limit;
            let local_posts = self.load_fav_posts_from_db(&opt).await?;
            if local_posts.is_empty() {
                break;
            }

            let subtask_name = format!("{task_name}_{index}");
            options.export_task_name = subtask_name;
            let html = self.processer.generate_html(local_posts, &options).await?;
            self.exporter.export_page(html, &options).await?;

            let task_progress = TaskProgress {
                id: 0,
                total_increment: 0,
                current_increment: limit as u64,
            };
            self.msg_sender
                .send(Message::TaskProgress(task_progress))
                .await?;
            offset += limit;
            index += 1;
            if offset >= posts_sum {
                break;
            }
        }
        Ok(())
    }

    // export favorite posts from weibo
    pub(super) async fn backup_favorites(&self, options: BFOptions) -> Result<()> {
        let range = options.range.to_owned();
        assert!(range.start() != &0);
        info!("favorites download range is {range:?}");
        let mut total_downloaded: usize = 0;
        let page_range = *range.start() / 20 + 1..=*range.end() / 20;
        let last_page = page_range.end() - 1;
        let total_pages = (page_range.end() - page_range.start() + 1) as f32;

        for (i, page) in page_range.into_iter().enumerate() {
            let posts_sum = self.backup_one_fav_page(page, options.clone()).await?;
            total_downloaded += posts_sum;
            info!("fetched {} posts in {}th page", posts_sum, page);

            // self.msg_sender
            //     .send(Message::InProgress(
            //         i as f32 / total_pages,
            //         format!("已下载第{page}页...耐心等待，先干点别的"),
            //     ))
            //     .await?;
            if i != last_page as usize {
                sleep(BACKUP_TASK_INTERVAL).await;
            }
        }
        info!("fetched {total_downloaded} posts in total");
        Ok(())
    }
}
