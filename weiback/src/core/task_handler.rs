use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use futures::stream::{self, StreamExt, TryStreamExt};
use log::{debug, error, info};
use tokio::time::sleep;

use super::post_processer::PostProcesser;
use super::task::TaskContext;
use crate::api::{ApiClient, ContainerType};
use crate::core::task::{
    BackupFavoritesOptions, BackupUserPostsOptions, ExportJobOptions, PaginatedPostInfo, PostQuery,
};
use crate::emoji_map::EmojiMap;
use crate::error::Result;
use crate::exporter::Exporter;
use crate::html_generator::HTMLGenerator;
use crate::media_downloader::MediaDownloader;
use crate::message::TaskType;
use crate::models::{Picture, PictureMeta, User};
use crate::storage::{PictureInfo, Storage};
use crate::utils::make_page_name;

#[derive(Debug, Clone)]
pub struct TaskHandler<A: ApiClient, S: Storage, E: Exporter, D: MediaDownloader> {
    api_client: A,
    storage: S,
    exporter: E,
    downloader: D,
    processer: PostProcesser<A, S, D>,
    html_generator: HTMLGenerator<A, S>,
}

impl<A: ApiClient, S: Storage, E: Exporter, D: MediaDownloader> TaskHandler<A, S, E, D> {
    pub fn new(api_client: A, storage: S, exporter: E, downloader: D) -> Result<Self> {
        let emoji_map = EmojiMap::new(api_client.clone());

        let processer = PostProcesser::new(storage.clone(), downloader.clone(), emoji_map.clone())?;

        let html_generator = HTMLGenerator::new(emoji_map, storage.clone());

        Ok(TaskHandler {
            api_client,
            storage,
            exporter,
            downloader,
            processer,
            html_generator,
        })
    }

    pub async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        self.storage.get_user(uid).await
    }

    pub async fn get_picture_blob(&self, ctx: Arc<TaskContext>, id: &str) -> Result<Option<Bytes>> {
        let infos = self.storage.get_pictures_by_id(id).await?;
        if let Some(info) = self.choose_best_picture(infos) {
            self.storage.get_picture_blob(ctx, info.meta.url()).await
        } else {
            Ok(None)
        }
    }

    fn choose_best_picture(&self, infos: Vec<PictureInfo>) -> Option<PictureInfo> {
        // Placeholder logic: just take the first one.
        // TODO: Implement proper resolution priority logic here.
        infos.into_iter().next()
    }

    pub async fn save_user_info(&self, ctx: Arc<TaskContext>, user: &User) -> Result<()> {
        info!("Saving user info for user id: {}", user.id);
        self.storage.save_user(user).await?;
        info!(
            "User {} with name {} saved to db",
            user.id, user.screen_name
        );

        let avatar_url = user.avatar_hd.clone();

        if self.storage.picture_saved(ctx.clone(), &avatar_url).await? {
            return Ok(());
        }
        let user_id = user.id;
        let pic_meta = PictureMeta::avatar(avatar_url.as_str(), user_id)?;
        let storage = self.storage.clone();

        let callback = Box::new(
            move |ctx: Arc<TaskContext>,
                  blob: Bytes|
                  -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
                Box::pin(async move {
                    let pic = Picture {
                        meta: pic_meta,
                        blob,
                    };
                    storage.save_picture(ctx, &pic).await?;
                    Ok(())
                })
            },
        );

        // Using task_id 0 as this is not a user-initiated task with progress tracking.
        self.downloader
            .download_media(ctx, &avatar_url, callback)
            .await?;

        Ok(())
    }

    async fn backup_procedure<F, Fut>(
        &self,
        ctx: Arc<TaskContext>,
        num_pages: u32,
        task_type: TaskType,
        page_backup_fn: F,
    ) -> Result<()>
    where
        F: Fn(u32) -> Fut,
        Fut: Future<Output = Result<usize>>,
    {
        info!(
            "Starting backup procedure for task {}, type: {:?}",
            ctx.task_id, task_type
        );
        let task_interval = ctx.config.backup_task_interval;

        let mut total_downloaded: usize = 0;
        let start = 1;
        let end = num_pages + start;
        debug!(
            "Backup task {} page range: {}..={}",
            ctx.task_id, start, num_pages
        );
        ctx.send_progress(task_type.clone(), num_pages as u64, 0)
            .await?;

        for page in start..=end {
            let posts_sum = page_backup_fn(page).await.map_err(|e| {
                error!(
                    "Failed to backup page {} for task {}: {}",
                    page, ctx.task_id, e
                );
                e
            })?;

            total_downloaded += posts_sum;
            info!(
                "fetched {posts_sum} posts in {}th page ({}/{})",
                page,
                page - start + 1,
                num_pages
            );

            ctx.send_progress(task_type.clone(), 0, 1).await?;
            if page != end {
                sleep(task_interval).await;
            }
        }
        info!(
            "Backup procedure for task {} finished. Fetched {} posts in total",
            ctx.task_id, total_downloaded
        );
        Ok(())
    }

    // backup user posts
    pub(super) async fn backup_user(
        &self,
        ctx: Arc<TaskContext>,
        options: BackupUserPostsOptions,
    ) -> Result<()> {
        let uid = options.uid;
        let container_type = options.backup_type.into();
        info!(
            "Start backing up user {uid} posts, type: {:?}, from 1 to {}",
            options.backup_type, options.num_pages
        );

        self.backup_procedure(ctx.clone(), options.num_pages, TaskType::BackUser, |page| {
            self.backup_one_page(ctx.clone(), uid, page, container_type)
        })
        .await?;

        info!("Finished backing up user {uid} posts.");
        Ok(())
    }

    // backup one page of posts of the user
    async fn backup_one_page(
        &self,
        ctx: Arc<TaskContext>,
        uid: i64,
        page: u32,
        container_type: ContainerType,
    ) -> Result<usize> {
        debug!(
            "Backing up page {page} for user {uid}, task {}",
            ctx.task_id
        );
        let posts = self
            .api_client
            .profile_statuses(uid, page, container_type)
            .await?;
        let result = posts.len();
        self.processer.process(ctx, posts).await?;
        Ok(result)
    }

    // export favorite posts from weibo
    pub(super) async fn backup_favorites(
        &self,
        ctx: Arc<TaskContext>,
        options: BackupFavoritesOptions,
    ) -> Result<()> {
        info!("Start backing up favorites from 1 to {}", options.num_pages);

        self.backup_procedure(ctx.clone(), options.num_pages, TaskType::BackFav, |page| {
            self.backup_one_fav_page(ctx.clone(), page)
        })
        .await?;
        info!("Finished backing up favorites.");
        Ok(())
    }

    // backup one page of favorites
    async fn backup_one_fav_page(&self, ctx: Arc<TaskContext>, page: u32) -> Result<usize> {
        debug!("Backing up favorites page {page}, task {}", ctx.task_id);
        let posts = self.api_client.favorites(page).await?;
        let result = posts.len();
        let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
        self.processer.process(ctx, posts).await?;

        // call mark_user_backed_up after all posts inserted, to ensure the post is in db
        for id in ids {
            self.storage.mark_post_favorited(id).await?;
        }
        Ok(result)
    }

    // unfavorite all posts that are in weibo favorites
    pub(super) async fn unfavorite_posts(&self, ctx: Arc<TaskContext>) -> Result<()> {
        info!("Starting unfavorite posts task {}", ctx.task_id);
        let task_interval = ctx.config.other_task_interval;
        let ids = self.storage.get_posts_id_to_unfavorite().await?;
        let len = ids.len();
        info!("Found {len} posts to unfavorite");
        ctx.send_progress(TaskType::Unfav, len as u64, 0).await?;
        for (i, id) in ids.into_iter().enumerate() {
            if let Err(e) = self.api_client.favorites_destroy(id).await {
                error!("Failed to unfavorite post {id}: {e}");
                continue;
            }
            self.storage.mark_post_unfavorited(id).await?;
            info!("Post {id} ({i}/{len})unfavorited successfully");
            ctx.send_progress(TaskType::Unfav, 0, 1).await?;
            if i < len - 1 {
                tokio::time::sleep(task_interval).await;
            }
        }
        info!("Unfavorite posts task {} finished", ctx.task_id);
        Ok(())
    }

    pub async fn export_posts(
        &self,
        ctx: Arc<TaskContext>,
        options: ExportJobOptions,
    ) -> Result<()> {
        info!("Starting export from local with options: {options:?}");
        let posts_per_page = crate::config::get_config().read()?.posts_per_html;
        let mut query = options.query;
        query.posts_per_page = posts_per_page;

        for page_index in 1.. {
            query.page = page_index;
            let local_posts = self.storage.query_posts(query.clone()).await?;
            if local_posts.posts.is_empty() {
                info!("No more posts to export. Exiting loop.");
                break;
            }

            let page_name = make_page_name(&options.output.task_name, page_index as i32);
            let html = self
                .html_generator
                .generate_html(ctx.clone(), local_posts.posts, &page_name)
                .await?;
            self.exporter
                .export_page(html, &page_name, &options.output.export_dir)
                .await?;
        }
        info!("Finished exporting from local");
        Ok(())
    }

    pub async fn query_posts(
        &self,
        ctx: Arc<TaskContext>,
        query: PostQuery,
    ) -> Result<PaginatedPostInfo> {
        info!("Querying local posts with query: {:?}", query);
        let paginated_posts = self.storage.query_posts(query).await?;
        let posts_info = stream::iter(paginated_posts.posts)
            .map(|post| self.processer.build_post_info(ctx.clone(), post))
            .buffered(5)
            .try_collect()
            .await?;

        Ok(PaginatedPostInfo {
            posts: posts_info,
            total_items: paginated_posts.total_items,
        })
    }

    pub async fn delete_post(&self, ctx: Arc<TaskContext>, id: i64) -> Result<()> {
        self.storage.delete_post(ctx, id).await
    }

    pub async fn rebackup_post(&self, ctx: Arc<TaskContext>, id: i64) -> Result<()> {
        let post = self.api_client.statuses_show(id).await?;
        self.processer.process(ctx, vec![post]).await
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use tokio::sync::mpsc;
    use weibosdk_rs::mock::MockClient;

    use super::*;
    use crate::{
        api::{FavoritesApi, ProfileStatusesApi},
        core::task::{BackupUserPostsOptions, ExportOutputConfig},
        mock::MockApi,
        mock::{exporter::MockExporter, media_downloader::MockMediaDownloader},
        models::Post,
        storage::{StorageImpl, database},
    };

    async fn create_test_storage() -> StorageImpl {
        let db_pool = database::create_db_pool_with_url(":memory:").await.unwrap();
        StorageImpl::new(db_pool)
    }

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
        posts.extend(
            api.profile_statuses(1786055427, 0, Default::default())
                .await
                .unwrap(),
        );
        posts
    }

    #[tokio::test]
    async fn test_backup_user() {
        let client = MockClient::new();
        let api_client = MockApi::new(client.clone());
        let storage = create_test_storage().await;
        let exporter = MockExporter::new();
        let downloader = MockMediaDownloader::new(true);
        let task_handler =
            TaskHandler::new(api_client.clone(), storage.clone(), exporter, downloader).unwrap();
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
        ids.reverse();

        let options = BackupUserPostsOptions {
            uid: 1786055427,
            num_pages: 1,
            backup_type: Default::default(),
        };
        let (msg_sender, _recv) = mpsc::channel(100);
        let dummy_context = Arc::new(TaskContext {
            task_id: 0,
            config: Default::default(),
            msg_sender,
        });
        task_handler
            .backup_user(dummy_context, options)
            .await
            .unwrap();
        let query = PostQuery {
            user_id: Some(uid),
            start_date: None,
            end_date: None,
            is_favorited: false,
            reverse_order: false,
            page: 1,
            posts_per_page: 1_000_000,
        };
        let ids_in_db = storage
            .query_posts(query)
            .await
            .unwrap()
            .posts
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();

        assert_eq!(ids_in_db, ids);
    }

    #[tokio::test]
    async fn test_backup_favorites() {
        let client = MockClient::new();
        let api_client = MockApi::new(client.clone());
        let storage = create_test_storage().await;
        let exporter = MockExporter::new();
        let downloader = MockMediaDownloader::new(true);
        let task_handler =
            TaskHandler::new(api_client.clone(), storage.clone(), exporter, downloader).unwrap();
        let posts = create_posts(&client, &api_client).await;
        let mut ids = posts
            .iter()
            .filter_map(|p| p.favorited.then_some(p.id))
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        ids.reverse();

        let options = BackupFavoritesOptions { num_pages: 1 };
        let (msg_sender, _recv) = mpsc::channel(100);
        let dummy_context = Arc::new(TaskContext {
            task_id: 0,
            config: Default::default(),
            msg_sender,
        });
        task_handler
            .backup_favorites(dummy_context, options)
            .await
            .unwrap();
        let query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            is_favorited: true,
            reverse_order: false,
            page: 1,
            posts_per_page: 1_000_000,
        };
        let ids_in_db = storage
            .query_posts(query)
            .await
            .unwrap()
            .posts
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>();
        assert_eq!(ids_in_db, ids);
    }

    #[tokio::test]
    async fn test_export_from_local() {
        let client = MockClient::new();
        let api_client = MockApi::new(client.clone());
        let storage = create_test_storage().await;
        let exporter = MockExporter::new();
        let downloader = MockMediaDownloader::new(true);
        let task_handler = TaskHandler::new(api_client, storage, exporter, downloader).unwrap();
        let export_dir = Path::new("export_dir").into();
        let task_name = "test".to_string();

        let options = ExportJobOptions {
            query: PostQuery {
                user_id: None,
                start_date: None,
                end_date: None,
                is_favorited: true,
                reverse_order: false,
                page: 1,
                posts_per_page: 20,
            },
            output: ExportOutputConfig {
                task_name,
                export_dir,
            },
        };
        let (msg_sender, _recv) = mpsc::channel(100);
        let dummy_context = Arc::new(TaskContext {
            task_id: 0,
            config: Default::default(),
            msg_sender,
        });
        task_handler
            .export_posts(dummy_context, options)
            .await
            .unwrap();
    }
}
