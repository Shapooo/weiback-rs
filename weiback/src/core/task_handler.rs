//! This module implements the high-level logic for all application tasks.
//!
//! The [`TaskHandler`] coordinates between the [`ApiClient`], [`Storage`], [`Exporter`],
//! and [`PostProcesser`] to fulfill requests such as:
//! - Backing up a user's entire post history.
//! - Synchronizing favorited posts.
//! - Exporting saved posts to HTML.
//! - Cleaning up redundant media or invalid avatars.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use futures::{
    pin_mut,
    stream::{self, StreamExt, TryStreamExt},
};
use tokio::{fs, time::sleep};
use tracing::{debug, error, info, warn};
use url::Url;

use super::post_processer::PostProcesser;
use super::task::{
    BackupFavoritesOptions, BackupUserPostsOptions, CleanupInvalidPostsOptions,
    CleanupPicturesOptions, DeletePostOptions, ExportJobOptions, PaginatedPostInfo, PostQuery,
    ResolutionPolicy, TaskContext,
};
use super::task_manager::{TaskError, TaskErrorType};
use crate::emoji_map::EmojiMap;
use crate::error::{Error, Result};
use crate::exporter::Exporter;
use crate::html_generator::HTMLGenerator;
use crate::image_validator::{ImageStatus, ImageValidator};
use crate::media_downloader::MediaDownloader;
use crate::models::{Picture, PictureMeta, User};
use crate::storage::Storage;
use crate::utils::{make_page_name, pic_url_to_id};
use crate::{
    api::{ApiClient, ContainerType},
    storage::PictureInfo,
};

/// The primary executor for application tasks.
///
/// `TaskHandler` is responsible for fetching data from the API, processing it
/// (including media downloading), and managing its lifecycle within the local storage
/// and export systems.
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
    /// Creates a new `TaskHandler` instance.
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

    /// Retrieves a user from local storage by their UID.
    pub async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        self.storage.get_user(uid).await
    }

    /// Searches for users in local storage by screen name prefix.
    pub async fn search_users_by_screen_name_prefix(&self, prefix: &str) -> Result<Vec<User>> {
        self.storage
            .search_users_by_screen_name_prefix(prefix)
            .await
    }

    /// Retrieves the binary data of a picture from storage.
    ///
    /// This method automatically handles resolution fallback by sorting available
    /// picture definitions.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `id` - The picture ID.
    #[tracing::instrument(skip(self, ctx), fields(pic_id = %id), level = "info")]
    pub async fn get_picture_blob(&self, ctx: Arc<TaskContext>, id: &str) -> Result<Option<Bytes>> {
        let mut infos = self.storage.get_pictures_by_id(id).await?;
        infos.sort_by(|a, b| match (&a.meta, &b.meta) {
            (
                PictureMeta::Attached { definition: da, .. },
                PictureMeta::Attached { definition: db, .. },
            ) => db.cmp(da),
            _ => std::cmp::Ordering::Equal,
        });

        for info in infos {
            if let Some(blob) = self
                .storage
                .get_picture_blob(ctx.clone(), info.meta.url())
                .await?
            {
                return Ok(Some(blob));
            }
        }
        Ok(None)
    }

    /// Retrieves the binary data of a video from storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The video URL.
    #[tracing::instrument(skip(self, ctx), fields(video_url = %url), level = "info")]
    pub async fn get_video_blob(&self, ctx: Arc<TaskContext>, url: &str) -> Result<Option<Bytes>> {
        let url = Url::parse(url).map_err(|e| {
            error!("Failed to parse video URL: {}", e);
            Error::FormatError(format!("Invalid video URL: {}", url))
        })?;
        self.storage.get_video_blob(ctx, &url).await
    }

    /// Persists user information to the database and downloads their high-definition avatar.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `user` - The user object to save.
    #[tracing::instrument(skip(self, ctx, user), fields(uid = user.id, screen_name = %user.screen_name), level = "info")]
    pub async fn save_user_info(&self, ctx: Arc<TaskContext>, user: &User) -> Result<()> {
        self.storage.save_user(user).await?;

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

    /// Generic procedure for paginated backup tasks.
    ///
    /// Handles iteration, progress tracking, and intervals between requests.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `num_pages` - Number of pages to fetch.
    /// * `page_backup_fn` - An async closure that performs the actual backup of a single page.
    #[tracing::instrument(skip(self, ctx, page_backup_fn), fields(task_id = ctx.task_id))]
    async fn backup_procedure<F, Fut>(
        &self,
        ctx: Arc<TaskContext>,
        num_pages: u32,
        page_backup_fn: F,
    ) -> Result<()>
    where
        F: Fn(u32) -> Fut,
        Fut: Future<Output = Result<usize>>,
    {
        let task_interval = ctx.config.backup_task_interval;

        let mut total_downloaded: usize = 0;
        let start = 1;
        let end = num_pages + start;
        debug!(
            "Backup task {} page range: {}..={}",
            ctx.task_id.unwrap(),
            start,
            num_pages
        );
        ctx.task_manager.update_progress(0, num_pages as u64)?;

        let mut processed = 0;
        for page in start..end {
            let result = page_backup_fn(page).await;

            match result {
                Ok(posts_sum) => {
                    total_downloaded += posts_sum;
                    info!(
                        "fetched {posts_sum} posts in {}th page ({}/{})",
                        page,
                        page - start + 1,
                        num_pages
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to backup page {} for task {}: {}",
                        page,
                        ctx.task_id.unwrap(),
                        e
                    );
                    ctx.task_manager.report_task_error(TaskError {
                        error_type: TaskErrorType::DownloadMedia(format!("page {}", page)),
                        message: e.to_string(),
                    })?;
                }
            }
            processed += 1;
            ctx.task_manager
                .update_progress(processed, num_pages as u64)?;

            if page != end {
                sleep(task_interval).await;
            }
        }
        info!(
            "Backup procedure for task {} finished. Fetched {} posts in total",
            ctx.task_id.unwrap(),
            total_downloaded
        );
        Ok(())
    }

    /// Backs up posts for a specific user.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `options` - Configuration for the backup (UID, range, type).
    #[tracing::instrument(skip(self, ctx), fields(uid = options.uid, pages = options.num_pages), level = "info")]
    pub(super) async fn backup_user(
        &self,
        ctx: Arc<TaskContext>,
        options: BackupUserPostsOptions,
    ) -> Result<()> {
        let uid = options.uid;
        let container_type = options.backup_type.into();

        self.backup_procedure(ctx.clone(), options.num_pages, |page| {
            self.backup_one_page(ctx.clone(), uid, page, container_type)
        })
        .await?;

        info!("Finished backing up user {uid} posts.");
        Ok(())
    }

    /// Fetches and processes a single page of posts for a user.
    #[tracing::instrument(skip(self, ctx), fields(uid, page))]
    async fn backup_one_page(
        &self,
        ctx: Arc<TaskContext>,
        uid: i64,
        page: u32,
        container_type: ContainerType,
    ) -> Result<usize> {
        let count = ctx.config.posts_count;
        let posts = self
            .api_client
            .profile_statuses(uid, page, container_type, count)
            .await?;
        let result = posts.len();
        self.processer.process(ctx, posts).await?;
        Ok(result)
    }

    /// Backs up the current user's favorited posts.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `options` - Configuration for the backup (range).
    #[tracing::instrument(skip(self, ctx), fields(pages = options.num_pages), level = "info")]
    pub(super) async fn backup_favorites(
        &self,
        ctx: Arc<TaskContext>,
        options: BackupFavoritesOptions,
    ) -> Result<()> {
        self.backup_procedure(ctx.clone(), options.num_pages, |page| {
            self.backup_one_fav_page(ctx.clone(), page)
        })
        .await?;
        info!("Finished backing up favorites.");
        Ok(())
    }

    /// Fetches and processes a single page of favorite posts.
    async fn backup_one_fav_page(&self, ctx: Arc<TaskContext>, page: u32) -> Result<usize> {
        debug!(
            "Backing up favorites page {page}, task {}",
            ctx.task_id.unwrap()
        );
        let count = ctx.config.posts_count;
        let posts = self.api_client.favorites(page, count).await?;
        let result = posts.len();
        let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
        self.processer.process(ctx, posts).await?;

        // call mark_user_backed_up after all posts inserted, to ensure the post is in db
        for id in ids {
            self.storage.mark_post_favorited(id).await?;
        }
        Ok(result)
    }

    /// Unfavorites posts on Weibo that are currently present in local storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    #[tracing::instrument(skip(self, ctx), fields(task_id = ctx.task_id), level = "info")]
    pub(super) async fn unfavorite_posts(&self, ctx: Arc<TaskContext>) -> Result<()> {
        let task_interval = ctx.config.other_task_interval;
        let ids = self.storage.get_posts_id_to_unfavorite().await?;
        let len = ids.len();
        info!("Found {len} posts to unfavorite");
        ctx.task_manager.update_progress(0, len as u64)?;

        let mut processed: u64 = 0;
        for (i, id) in ids.into_iter().enumerate() {
            let result = self.api_client.favorites_destroy(id).await;

            match result {
                Ok(_) => {
                    self.storage.mark_post_unfavorited(id).await?;
                    info!("Post {id} ({i}/{len})unfavorited successfully");
                }
                Err(e) => {
                    error!("Failed to unfavorite post {id}: {e}");
                    ctx.task_manager.report_task_error(TaskError {
                        error_type: TaskErrorType::DownloadMedia(format!("unfavorite post {}", id)),
                        message: e.to_string(),
                    })?;
                }
            }
            processed += 1;
            ctx.task_manager.update_progress(processed, len as u64)?;

            if i < len - 1 {
                tokio::time::sleep(task_interval).await;
            }
        }
        info!("Unfavorite posts task {} finished", ctx.task_id.unwrap());
        Ok(())
    }

    /// Exports posts from local storage to an external format (HTML).
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `options` - Export configuration (query filters, output directory).
    #[tracing::instrument(skip(self, ctx), fields(task_name = %options.output.task_name), level = "info")]
    pub async fn export_posts(
        &self,
        ctx: Arc<TaskContext>,
        options: ExportJobOptions,
    ) -> Result<()> {
        let posts_per_page = crate::config::get_config().read()?.posts_per_html;
        let mut query = options.query.clone();
        query.posts_per_page = posts_per_page;

        // Get total count for progress tracking
        let mut count_query = options.query.clone();
        count_query.posts_per_page = 1;
        count_query.page = 1;
        let total_items = self.storage.query_posts(count_query).await?.total_items;
        let total_pages = total_items.div_ceil(posts_per_page as u64);
        info!(
            "Exporting {} posts total, {} pages",
            total_items, total_pages
        );
        ctx.task_manager.update_progress(0, total_pages)?;

        let mut processed: u64 = 0;
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

            processed += 1;
            if processed.is_multiple_of(10) {
                ctx.task_manager.update_progress(processed, total_pages)?;
            }
        }
        info!("Finished exporting from local");
        Ok(())
    }

    /// Queries local posts and enriches them with media/metadata.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `query` - Search and filter criteria.
    #[tracing::instrument(skip(self, _ctx), fields(query = ?query))]
    pub async fn query_posts(
        &self,
        _ctx: Arc<TaskContext>,
        query: PostQuery,
    ) -> Result<PaginatedPostInfo> {
        let paginated_posts = self.storage.query_posts(query).await?;
        let posts_info = stream::iter(paginated_posts.posts)
            .map(|post| self.processer.build_post_info(post))
            .buffered(5)
            .try_collect()
            .await?;

        Ok(PaginatedPostInfo {
            posts: posts_info,
            total_items: paginated_posts.total_items,
        })
    }

    /// Deletes a post from local storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `options` - The delete options including post id and deep/shallow mode.
    #[tracing::instrument(skip(self, ctx), level = "info")]
    pub async fn delete_post(
        &self,
        ctx: Arc<TaskContext>,
        options: DeletePostOptions,
    ) -> Result<()> {
        self.storage
            .delete_post(ctx, options.id, options.deep)
            .await
    }

    /// Re-fetches a batch of posts from Weibo API and processes them.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `query` - The `PostQuery` to select posts to be re-backed up.
    #[tracing::instrument(skip(self, ctx), fields(query = ?query), level = "info")]
    pub(super) async fn rebackup_posts(
        &self,
        ctx: Arc<TaskContext>,
        query: PostQuery,
    ) -> Result<()> {
        let ids = self.storage.query_all_post_ids(query).await.map_err(|e| {
            error!(
                "Failed to query post IDs for task {}: {}",
                ctx.task_id.unwrap(),
                e
            );
            e
        })?;
        let total = ids.len();
        info!("Found {} posts to re-backup", total);
        ctx.task_manager.update_progress(0, total as u64)?;

        let task_interval = ctx.config.backup_task_interval;
        let mut processed: u64 = 0;
        for (i, id) in ids.into_iter().enumerate() {
            let post_result = self.api_client.statuses_show(id).await;
            let process_result = match post_result {
                Ok(post) => self.processer.process(ctx.clone(), vec![post]).await,
                Err(e) => {
                    error!(
                        "Failed to fetch post {} for task {}: {}",
                        id,
                        ctx.task_id.unwrap(),
                        e
                    );
                    Err(e)
                }
            };

            match process_result {
                Ok(_) => {
                    info!("re-backed up post {} ({}/{})", id, i + 1, total);
                }
                Err(e) => {
                    error!(
                        "Failed to process re-backed up post {} for task {}: {}",
                        id,
                        ctx.task_id.unwrap(),
                        e
                    );
                    ctx.task_manager.report_task_error(TaskError {
                        error_type: TaskErrorType::DownloadMedia(format!("rebackup post {}", id)),
                        message: e.to_string(),
                    })?;
                }
            }
            processed += 1;
            if processed.is_multiple_of(100) {
                ctx.task_manager.update_progress(processed, total as u64)?;
            }

            if i < total - 1 {
                sleep(task_interval).await;
            }
        }
        info!("Finished re-backing up posts.");
        Ok(())
    }

    /// Cleans up redundant picture files based on the specified resolution policy.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `options` - Cleanup configuration (Highest/Lowest resolution).
    #[tracing::instrument(skip(self, ctx), fields(policy = ?options.policy), level = "info")]
    pub(super) async fn cleanup_pictures(
        &self,
        ctx: Arc<TaskContext>,
        options: CleanupPicturesOptions,
    ) -> Result<()> {
        info!(
            "Starting cleanup pictures task with policy: {:?}",
            options.policy
        );
        let ids = self.storage.get_duplicate_pic_ids().await?;
        let total = ids.len() as u64;
        info!("Found {} duplicate picture IDs", total);
        ctx.task_manager.update_progress(0, total)?;

        let mut processed: u64 = 0;
        for id in ids {
            let mut pictures = self.storage.get_pictures_by_id(&id).await?;
            if pictures.len() <= 1 {
                processed += 1;
                continue;
            }

            // Sort by definition
            pictures.sort_by(|a, b| match (&a.meta, &b.meta) {
                (
                    PictureMeta::Attached { definition: da, .. },
                    PictureMeta::Attached { definition: db, .. },
                ) => match options.policy {
                    ResolutionPolicy::Highest => db.cmp(da), // Highest resolution first
                    ResolutionPolicy::Lowest => da.cmp(db),  // Lowest resolution first
                },
                _ => std::cmp::Ordering::Equal,
            });

            // Keep the first one, delete the rest
            for pic in pictures.into_iter().skip(1) {
                if let Err(e) = self
                    .storage
                    .delete_picture(ctx.clone(), pic.meta.url())
                    .await
                {
                    error!(
                        "Failed to delete redundant picture {}: {}",
                        pic.meta.url(),
                        e
                    );
                    ctx.task_manager.report_task_error(TaskError {
                        error_type: TaskErrorType::DownloadMedia(pic.meta.url().to_string()),
                        message: e.to_string(),
                    })?;
                }
            }

            processed += 1;
            if processed.is_multiple_of(100) {
                ctx.task_manager.update_progress(processed, total)?;
            }
        }
        info!("Finished cleanup pictures task");
        Ok(())
    }

    /// Identifies and removes invalid or outdated avatar files.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    #[tracing::instrument(skip(self, ctx), level = "info")]
    pub(super) async fn cleanup_outdated_avatars(&self, ctx: Arc<TaskContext>) -> Result<()> {
        info!("Starting cleanup invalid avatars task");
        let duplicate_uids = self.storage.get_users_with_duplicate_avatars().await?;
        let total = duplicate_uids.len() as u64;
        ctx.task_manager.update_progress(0, total)?;

        let users = self.storage.get_users_by_ids(&duplicate_uids).await?;
        let avatar_map: std::collections::HashMap<i64, String> = users
            .into_iter()
            .filter_map(|u| pic_url_to_id(&u.avatar_hd).ok().map(|id| (u.id, id)))
            .collect();

        let mut processed: u64 = 0;
        for user_id in duplicate_uids {
            let current_id = avatar_map.get(&user_id);
            if let Some(current_id) = current_id {
                let avatar_infos = self.storage.get_avatar_infos(user_id).await?;
                for info in avatar_infos {
                    let pic_id = match pic_url_to_id(info.meta.url()) {
                        Ok(id) => id,
                        Err(e) => {
                            error!("Failed to parse avatar URL: {}", e);
                            continue;
                        }
                    };
                    if pic_id != *current_id {
                        info!("Deleting invalid avatar: {} for user {}", pic_id, user_id);
                        if let Err(e) = self
                            .storage
                            .delete_picture(ctx.clone(), info.meta.url())
                            .await
                        {
                            error!("Failed to delete invalid avatar {}: {}", pic_id, e);
                            ctx.task_manager.report_task_error(TaskError {
                                error_type: TaskErrorType::DownloadMedia(
                                    info.meta.url().to_string(),
                                ),
                                message: e.to_string(),
                            })?;
                        }
                    }
                }
            }
            processed += 1;
            if processed.is_multiple_of(100) {
                ctx.task_manager.update_progress(processed, total)?;
            }
        }
        info!("Finished cleanup invalid avatars task");
        Ok(())
    }

    /// Cleans up invalid posts based on the specified options.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `options` - Cleanup configuration.
    #[tracing::instrument(skip(self, ctx), fields(clean_retweeted_invalid = options.clean_retweeted_invalid), level = "info")]
    pub(super) async fn cleanup_invalid_posts(
        &self,
        ctx: Arc<TaskContext>,
        options: CleanupInvalidPostsOptions,
    ) -> Result<()> {
        info!(
            "Starting cleanup invalid posts task with options: {:?}",
            options
        );
        let ids = self
            .storage
            .get_invalid_posts_ids(options.clean_retweeted_invalid)
            .await?;
        let total = ids.len() as u64;
        info!("Found {} invalid posts to clean up", total);
        ctx.task_manager.update_progress(0, total)?;

        let mut processed: u64 = 0;
        for id in ids {
            if let Err(e) = self.storage.delete_post(ctx.clone(), id, true).await {
                error!("Failed to delete invalid post {}: {}", id, e);
                ctx.task_manager.report_task_error(TaskError {
                    error_type: TaskErrorType::DownloadMedia(format!("delete post {}", id)),
                    message: e.to_string(),
                })?;
            }
            processed += 1;
            if processed.is_multiple_of(100) {
                ctx.task_manager.update_progress(processed, total)?;
            }
        }
        info!("Finished cleanup invalid posts task");
        Ok(())
    }

    /// Re-fetches a single post from Weibo API and processes it.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `id` - The post ID to refresh.
    #[tracing::instrument(skip(self, ctx), level = "info")]
    pub async fn rebackup_post(&self, ctx: Arc<TaskContext>, id: i64) -> Result<()> {
        let post = self.api_client.statuses_show(id).await?;
        self.processer.process(ctx, vec![post]).await
    }

    /// Scans all local posts and re-backups those that have missing images.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `query` - Search criteria for posts to check.
    pub(super) async fn rebackup_missing_images(
        &self,
        ctx: Arc<TaskContext>,
        query: PostQuery,
    ) -> Result<()> {
        info!("Starting re-backup missing images task");
        let ids = self.storage.query_all_post_ids(query).await?;
        let total = ids.len();
        info!("Scanning {total} posts for missing images");
        ctx.task_manager.update_progress(0, total as u64)?;

        let task_interval = ctx.config.backup_task_interval;
        let mut processed: u64 = 0;
        for (i, id) in ids.into_iter().enumerate() {
            let post_opt = self.storage.get_post(id).await?;
            let post = match post_opt {
                Some(p) => p,
                None => {
                    processed += 1;
                    warn!("Post {} not found, skipping ({}/{})", id, i + 1, total);
                    continue;
                }
            };

            let has_missing = match self
                .processer
                .is_any_image_missing(ctx.clone(), &post)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    error!("Failed to check missing images for post {}: {}", id, e);
                    ctx.task_manager.report_task_error(TaskError {
                        error_type: TaskErrorType::DownloadMedia(format!(
                            "check images for post {}",
                            id
                        )),
                        message: e.to_string(),
                    })?;
                    info!("Scanned post {} ({}/{})", id, i + 1, total);
                    false
                }
            };

            if has_missing {
                info!("Post {id} has missing images, re-backing up...");
                let fetch_result = self.api_client.statuses_show(id).await;
                match fetch_result {
                    Ok(post) => {
                        if let Err(e) = self.processer.process(ctx.clone(), vec![post]).await {
                            error!("Failed to process re-backed up post {}: {}", id, e);
                            ctx.task_manager.report_task_error(TaskError {
                                error_type: TaskErrorType::DownloadMedia(format!(
                                    "rebackup post {}",
                                    id
                                )),
                                message: e.to_string(),
                            })?;
                        }
                        // Sleep between API calls to avoid rate limiting
                        sleep(task_interval).await;
                    }
                    Err(e) => {
                        error!("Failed to fetch post {} for re-backup: {}", id, e);
                        ctx.task_manager.report_task_error(TaskError {
                            error_type: TaskErrorType::DownloadMedia(format!("fetch post {}", id)),
                            message: e.to_string(),
                        })?;
                    }
                }
            }
            processed += 1;
            if processed.is_multiple_of(100) || has_missing {
                ctx.task_manager.update_progress(processed, total as u64)?;
            }
            info!("Scanned post {} ({}/{})", id, i + 1, total);
        }

        info!("Finished re-backup missing images task");
        Ok(())
    }

    /// Cleans up invalid pictures (e.g., "image deleted" placeholders) from local storage.
    ///
    /// This function:
    /// 1. Loads pictures in batches from the database
    /// 2. Checks file size - if > 15kB, skips (can't be invalid placeholder)
    /// 3. If <= 15kB, tries to parse the image - if can't parse, it's invalid
    /// 4. Otherwise, uses ImageValidator to check if it's invalid
    /// 5. Deletes invalid images from both filesystem and database
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    pub(super) async fn cleanup_invalid_pictures(&self, ctx: Arc<TaskContext>) -> Result<()> {
        info!("Starting cleanup invalid pictures task");

        // Get total count first for progress tracking
        let total = self.storage.count_pictures().await?;
        info!("Found {} pictures to check", total);
        ctx.task_manager.update_progress(0, total)?;
        let mut processed: u64 = 0;

        let mut deleted_count: u64 = 0;
        let storage = self.storage.clone();

        // Use the lazy stream to iterate over pictures
        let picture_stream = storage.get_all_pictures();
        pin_mut!(picture_stream);
        while let Some(pic_info_result) = picture_stream.next().await {
            // Process the picture, report error but continue if failed
            let deleted = self
                .process_picture_for_cleanup(ctx.clone(), pic_info_result)
                .await?;

            if deleted {
                deleted_count += 1;
            }
            processed += 1;
            if processed.is_multiple_of(200) {
                ctx.task_manager.update_progress(processed, total)?;
            }
        }

        info!(
            "Finished cleanup invalid pictures task. Processed: {}, Deleted: {}",
            total, deleted_count
        );
        Ok(())
    }

    /// Processes a single picture for cleanup, checking if it's invalid and deleting if so.
    ///
    /// Returns `Ok(true)` if the picture was deleted, `Ok(false)` if it was kept,
    /// or `Err` if an error occurred during processing.
    async fn process_picture_for_cleanup(
        &self,
        ctx: Arc<TaskContext>,
        pic_info_result: Result<PictureInfo>,
    ) -> Result<bool> {
        const MAX_FILE_SIZE: u64 = 15 * 1024; // 15kB

        let pic_info = pic_info_result?;
        let url = pic_info.meta.url();
        let picture_path = &ctx.config.picture_path;
        let file_path = picture_path.join(&pic_info.path);

        // Check if file exists
        let metadata = fs::metadata(&file_path).await?;
        let file_size = metadata.len();

        // Step 1: If file > 15kB, skip (can't be invalid placeholder)
        if file_size > MAX_FILE_SIZE {
            return Ok(false);
        }

        // Step 2: Read the file
        let data = fs::read(&file_path).await?;

        // Step 3: Decode and check if the image is invalid (censored or unparseable)
        let status = ImageValidator::is_invalid_weibo_image(&data);

        if status.is_invalid() {
            let reason = match status {
                ImageStatus::Censored => "censored (placeholder)",
                ImageStatus::Unparseable => "unparseable",
                ImageStatus::Valid => unreachable!(),
            };
            info!("Image is {}: {}", reason, file_path.display());

            self.storage.delete_picture_by_url(ctx, url).await?;

            return Ok(true);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use weibosdk_rs::mock::MockClient;

    use super::*;
    use crate::{
        api::{FavoritesApi, ProfileStatusesApi},
        core::{
            task::{BackupUserPostsOptions, ExportOutputConfig},
            task_manager::{TaskManager, TaskType},
        },
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
        let mut posts = api.favorites(0, 20).await.unwrap();
        posts.extend(
            api.profile_statuses(1786055427, 0, Default::default(), 20)
                .await
                .unwrap(),
        );
        posts
    }

    fn create_dummy_ctx() -> Arc<TaskContext> {
        let task_manager = Arc::new(TaskManager::new());
        task_manager
            .start_task(0, TaskType::Export, "test".into(), 0)
            .unwrap();

        Arc::new(TaskContext {
            task_id: Some(0),
            config: Default::default(),
            task_manager,
        })
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
        let dummy_context = create_dummy_ctx();
        task_handler
            .backup_user(dummy_context, options)
            .await
            .unwrap();
        let query = PostQuery {
            user_id: Some(uid),
            start_date: None,
            end_date: None,
            search_term: None,
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
        let dummy_context = create_dummy_ctx();
        task_handler
            .backup_favorites(dummy_context, options)
            .await
            .unwrap();
        let query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            search_term: None,
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
                search_term: None,
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
        let dummy_context = create_dummy_ctx();
        task_handler
            .export_posts(dummy_context, options)
            .await
            .unwrap();
    }
}
