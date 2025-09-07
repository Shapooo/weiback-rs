use log::{debug, error, info};
use std::future::Future;
use tokio::{self, sync::mpsc::Sender, time::sleep};

use super::post_processer::PostProcesser;
use crate::api::ApiClient;
use crate::config::get_config;
use crate::core::task::{BFOptions, BUOptions, ExportOptions};
use crate::emoji_map::EmojiMap;
use crate::error::Result;
use crate::exporter::Exporter;
use crate::html_generator::{HTMLGenerator, create_tera};
use crate::media_downloader::MediaDownloader;
use crate::message::{Message, TaskProgress, TaskType};
use crate::models::Post;
use crate::storage::Storage;
use crate::utils::make_page_name;

#[derive(Debug, Clone)]
pub struct TaskHandler<A: ApiClient, S: Storage, E: Exporter, D: MediaDownloader> {
    api_client: A,
    storage: S,
    exporter: E,
    processer: PostProcesser<A, S, D>,
    html_generator: HTMLGenerator<A, S, D>,
    msg_sender: Sender<Message>,
}

impl<A: ApiClient, S: Storage, E: Exporter, D: MediaDownloader> TaskHandler<A, S, E, D> {
    pub fn new(
        api_client: A,
        storage: S,
        exporter: E,
        downloader: D,
        msg_sender: Sender<Message>,
    ) -> Result<Self> {
        let emoji_map = EmojiMap::new(api_client.clone());

        let processer = PostProcesser::new(storage.clone(), downloader.clone(), emoji_map.clone())?;

        let tera = create_tera(crate::config::get_config().read()?.templates_path.as_path())?;
        let html_generator = HTMLGenerator::new(emoji_map, storage.clone(), downloader, tera);

        Ok(TaskHandler {
            api_client,
            storage,
            exporter,
            processer,
            html_generator,
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
        let count = get_config().read()?.sdk_config.status_count as u32;
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
        let count = get_config().read()?.sdk_config.fav_count as u32;
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

            let page_name = make_page_name(&options.task_name, index);
            let html = self
                .html_generator
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tokio::sync::mpsc;
    use weibosdk_rs::mock::MockClient;

    use super::*;
    use crate::{
        api::{FavoritesApi, ProfileStatusesApi},
        core::task::BUOptions,
        mock::MockApi,
        mock::{
            exporter::MockExporter, media_downloader::MockMediaDownloader, storage::MockStorage,
        },
    };

    async fn create_posts(client: &MockClient, api: &MockApi) -> Vec<Post> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fav_path = manifest_dir.join("tests/data/favorites.json");
        client.set_favorites_response_from_file(&fav_path).unwrap();
        let prof_path = manifest_dir.join("tests/data/profile_statuses.json");
        client
            .set_profile_statuses_response_from_file(&prof_path)
            .unwrap();
        let statuses_show_path = manifest_dir.join("tests/data/statuses_show.json");
        client
            .set_statuses_show_response_from_file(&statuses_show_path)
            .unwrap();
        let mut posts = api.favorites(0).await.unwrap();
        posts.extend(api.profile_statuses(1786055427, 0).await.unwrap());
        posts
    }

    #[tokio::test]
    async fn test_backup_user() {
        let client = MockClient::new();
        let api_client = MockApi::new(client.clone());
        let storage = MockStorage::new();
        let exporter = MockExporter::new();
        let downloader = MockMediaDownloader::new(true);
        let (msg_sender, _recv) = mpsc::channel(100);
        let task_handler = TaskHandler::new(
            api_client.clone(),
            storage.clone(),
            exporter,
            downloader,
            msg_sender,
        )
        .unwrap();
        let uid = 1786055427;
        let posts = create_posts(&client, &api_client).await;
        let mut ids = posts
            .into_iter()
            .filter_map(|p| {
                if p.user.is_some() && p.user.as_ref().unwrap().id == uid {
                    Some(p.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        ids.sort();

        let options = BUOptions {
            uid: 1786055427,
            range: 1..=1,
        };
        task_handler.backup_user(1, options).await.unwrap();
        let ids_in_db = storage
            .get_ones_posts(uid, 0..=100000, false)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();

        assert_eq!(ids_in_db, ids);
    }

    #[tokio::test]
    async fn test_backup_favorites() {
        let client = MockClient::new();
        let api_client = MockApi::new(client.clone());
        let storage = MockStorage::new();
        let exporter = MockExporter::new();
        let downloader = MockMediaDownloader::new(true);
        let (msg_sender, _recv) = mpsc::channel(100);
        let task_handler = TaskHandler::new(
            api_client.clone(),
            storage.clone(),
            exporter,
            downloader,
            msg_sender,
        )
        .unwrap();
        let posts = create_posts(&client, &api_client).await;
        let mut ids = posts
            .iter()
            .filter_map(|p| p.favorited.then_some(p.id))
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();

        let options = BFOptions { range: 1..=1 };
        task_handler.backup_favorites(1, options).await.unwrap();
        let ids_in_db = storage
            .get_favorites(0..=100000, false)
            .await
            .unwrap()
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        assert_eq!(ids_in_db, ids);
    }

    #[tokio::test]
    async fn test_export_from_local() {
        let client = MockClient::new();
        let api_client = MockApi::new(client.clone());
        let storage = MockStorage::new();
        let exporter = MockExporter::new();
        let downloader = MockMediaDownloader::new(true);
        let (msg_sender, _recv) = mpsc::channel(100);
        let task_handler =
            TaskHandler::new(api_client, storage, exporter, downloader, msg_sender).unwrap();
        let export_dir = Path::new("export_dir").into();
        let task_name = "test".to_string();

        let options = ExportOptions {
            task_name,
            range: 1..=20,
            reverse: false,
            export_dir,
        };
        task_handler.export_from_local(options).await.unwrap();
    }
}
