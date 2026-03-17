//! This module defines the storage layer for the application.
//! It provides a unified interface (`Storage` trait) for interacting with
//! both the database (for metadata) and the file system (for media files).
//! The primary implementation is `StorageImpl`, which coordinates between
//! the SQLite database and file-system-based media storage.

pub mod database;
pub mod internal;
pub mod picture_storage;
pub mod video_storage;

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{
    Stream, TryFutureExt,
    stream::{self, StreamExt},
};
use itertools::Itertools;
use picture_storage::FileSystemPictureStorage;
use sqlx::SqlitePool;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::core::task::{PaginatedPosts, PostQuery, TaskContext};
use crate::models::{Picture, PictureMeta, Post, User, Video};
use crate::{
    error::{Error, Result},
    storage::video_storage::FileSystemVideoStorage,
};
use internal::picture;
use internal::post::{self, PostInternal};
use internal::user;

/// Represents metadata and the associated file system path for a picture.
#[derive(Debug, Clone)]
pub struct PictureInfo {
    /// Metadata about the picture.
    pub meta: PictureMeta,
    /// The relative path to the picture file on the file system.
    pub path: PathBuf,
}

/// A trait defining the operations for storing and retrieving application data.
///
/// This trait abstracts over the underlying storage mechanisms, providing
/// methods for managing users, posts, and media (pictures and videos).
#[async_trait]
pub trait Storage: Send + Sync + Clone + 'static {
    /// Saves a user's information to the database.
    ///
    /// # Arguments
    /// * `user` - The user model to save.
    async fn save_user(&self, user: &User) -> Result<()>;

    /// Retrieves a user by their unique ID.
    ///
    /// # Arguments
    /// * `uid` - The unique identifier of the user.
    ///
    /// # Returns
    /// A `Result` containing an `Option<User>`.
    async fn get_user(&self, uid: i64) -> Result<Option<User>>;

    /// Retrieves multiple users by their unique IDs.
    ///
    /// # Arguments
    /// * `ids` - A slice of user IDs to retrieve.
    async fn get_users_by_ids(&self, ids: &[i64]) -> Result<Vec<User>>;

    /// Searches for users whose screen name starts with the given prefix.
    ///
    /// # Arguments
    /// * `prefix` - The screen name prefix to search for.
    async fn search_users_by_screen_name_prefix(&self, prefix: &str) -> Result<Vec<User>>;

    /// Saves a post to the database.
    ///
    /// # Arguments
    /// * `post` - The post model to save.
    async fn save_post(&self, post: &Post) -> Result<()>;

    /// Retrieves a post by its ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the post.
    ///
    /// # Returns
    /// A `Result` containing an `Option<Post>`.
    async fn get_post(&self, id: i64) -> Result<Option<Post>>;

    /// Queries posts based on various criteria.
    ///
    /// # Arguments
    /// * `query` - A `PostQuery` object containing search and filter criteria.
    ///
    /// # Returns
    /// A `Result` containing `PaginatedPosts`.
    async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPosts>;

    /// Queries posts based on various criteria and returns only their IDs.
    ///
    /// # Arguments
    /// * `query` - A `PostQuery` object containing search and filter criteria.
    ///
    /// # Returns
    /// A `Result` containing a vector of post IDs.
    async fn query_all_post_ids(&self, query: PostQuery) -> Result<Vec<i64>>;

    /// Marks a post as unfavorited in the database.
    ///
    /// # Arguments
    /// * `id` - The ID of the post to mark.
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()>;

    /// Marks a post as favorited in the database.
    ///
    /// # Arguments
    /// * `id` - The ID of the post to mark.
    async fn mark_post_favorited(&self, id: i64) -> Result<()>;

    /// Retrieves IDs of posts that are marked for unfavoriting.
    ///
    /// # Returns
    /// A `Result` containing a vector of post IDs.
    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>>;

    /// Deletes a post and all its associated media from both the database and the file system.
    ///
    /// # Arguments
    /// * `ctx` - The task context containing configuration (like storage paths).
    /// * `id` - The ID of the post to delete.
    async fn delete_post(&self, ctx: Arc<TaskContext>, id: i64) -> Result<()>;

    /// Retrieves IDs of posts that are invalid (e.g., uid is NULL).
    ///
    /// # Arguments
    /// * `clean_retweeted_invalid` - Whether to include posts that are valid themselves
    /// but their retweeted content is invalid.
    async fn get_invalid_posts_ids(&self, clean_retweeted_invalid: bool) -> Result<Vec<i64>>;

    /// Saves a picture's content to the file system and its metadata to the database.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `picture` - The picture model containing metadata and binary data.
    async fn save_picture(&self, ctx: Arc<TaskContext>, picture: &Picture) -> Result<()>;

    /// Retrieves the absolute file system path for a given picture URL.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the picture.
    ///
    /// # Returns
    /// A `Result` containing an `Option<PathBuf>`.
    async fn get_picture_path(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<Option<PathBuf>>;

    /// Retrieves information for all pictures attached to a specific post.
    ///
    /// # Arguments
    /// * `post_id` - The ID of the post.
    async fn get_attached_infos(&self, post_id: i64) -> Result<Vec<PictureInfo>>;

    /// Retrieves information for a user's primary avatar.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user.
    async fn get_avatar_info(&self, user_id: i64) -> Result<Option<PictureInfo>>;

    /// Retrieves information for all avatars associated with a user.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user.
    async fn get_avatar_infos(&self, user_id: i64) -> Result<Vec<PictureInfo>>;

    /// Finds users who have duplicate avatar entries in the database.
    ///
    /// # Returns
    /// A `Result` containing a vector of user IDs.
    async fn get_users_with_duplicate_avatars(&self) -> Result<Vec<i64>>;

    /// Retrieves picture information for a list of picture IDs.
    ///
    /// # Arguments
    /// * `ids` - A slice of picture IDs (URLs or keys).
    async fn get_pictures_by_ids(&self, ids: &[String]) -> Result<Vec<PictureInfo>>;

    /// Retrieves picture information for a specific picture ID.
    ///
    /// # Arguments
    /// * `id` - The picture ID.
    async fn get_pictures_by_id(&self, id: &str) -> Result<Vec<PictureInfo>>;

    /// Finds IDs of pictures that have duplicate entries in the database.
    ///
    /// # Returns
    /// A `Result` containing a vector of picture IDs.
    async fn get_duplicate_pic_ids(&self) -> Result<Vec<String>>;

    /// Retrieves pictures with paths in batches for cleanup operations.
    ///
    /// Returns a lazy stream that automatically handles pagination.
    // fn pictures(self: Arc<Self>) -> Pin<Box<dyn Stream<Item = Result<PictureInfo>> + Send>>;
    fn get_all_pictures(&self) -> impl Stream<Item = Result<PictureInfo>> + Send + '_;

    /// Counts the total number of pictures with paths in the database.
    async fn count_pictures(&self) -> Result<u64>;

    /// Deletes a specific picture from both the file system and the database.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the picture to delete.
    async fn delete_picture(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<()>;

    /// Deletes a specific picture by URL from both the file system and the database.
    /// This is a simplified version that takes ctx directly.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the picture to delete.
    async fn delete_picture_by_url(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<()>;

    /// Retrieves the binary content (blob) of a picture from the storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the picture.
    ///
    /// # Returns
    /// A `Result` containing an `Option<Bytes>`.
    async fn get_picture_blob(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
    ) -> Result<Option<bytes::Bytes>>;

    /// Checks if a picture is already saved in the storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the picture.
    async fn picture_saved(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<bool>;

    /// Saves a video's content to the file system and its metadata to the database.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `video` - The video model.
    async fn save_video(&self, ctx: Arc<TaskContext>, video: &Video) -> Result<()>;

    /// Retrieves the binary content (blob) of a video from the storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the video.
    ///
    /// # Returns
    /// A `Result` containing an `Option<Bytes>`.
    async fn get_video_blob(
        &self,
        ctx: Arc<TaskContext>,
        url: &Url,
    ) -> Result<Option<bytes::Bytes>>;

    /// Checks if a video is already saved in the storage.
    ///
    /// # Arguments
    /// * `ctx` - The task context.
    /// * `url` - The URL of the video.
    async fn video_saved(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<bool>;
}

/// The default implementation of the `Storage` trait.
///
/// It uses a SQLite database pool for metadata management and specialized
/// file system storage handlers for pictures and videos.
#[derive(Debug, Clone)]
pub struct StorageImpl {
    db_pool: SqlitePool,
    pic_storage: FileSystemPictureStorage,
    video_storage: FileSystemVideoStorage,
}

impl StorageImpl {
    /// Creates a new `StorageImpl` instance with the given database pool.
    pub fn new(db_pool: SqlitePool) -> Self {
        info!("Storage initialized successfully.");
        StorageImpl {
            db_pool,
            pic_storage: Default::default(),
            video_storage: Default::default(),
        }
    }

    /// Recursively saves a post and its associated user and retweeted status.
    fn _save_post(&self, post: Post) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            debug!("Saving post with id: {}", post.id);
            if let Some(user) = &post.user {
                user::save_user(&self.db_pool, user).await?;
            }
            if let Some(ret_post) = post.retweeted_status.as_deref() {
                self._save_post(ret_post.clone()).await?;
            }
            let post_storage: PostInternal = post.try_into()?;
            match post::save_post(&self.db_pool, &post_storage).await {
                Ok(()) => {
                    debug!("Post with id: {} saved successfully", post_storage.id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to save post with id: {}: {:?}", post_storage.id, e);
                    Err(e)
                }
            }
        })
    }

    /// Retrieves a post and hydrates it with user and retweeted post.
    fn get_post(&self, id: i64) -> Pin<Box<dyn Future<Output = Result<Option<Post>>> + Send + '_>> {
        Box::pin(async move {
            if let Some(post) = post::get_post(&self.db_pool, id).await? {
                self.hydrate_post(post).await.map(Some)
            } else {
                Ok(None)
            }
        })
    }

    /// Converts a internal post representation into a full `Post` model,
    /// fetching the associated user and retweeted post from the database.
    async fn hydrate_post(&self, post: PostInternal) -> Result<Post> {
        let user = if let Some(uid) = post.uid {
            user::get_user(&self.db_pool, uid).await?
        } else {
            None
        };

        let retweeted_status = if let Some(retweeted_id) = post.retweeted_id {
            Some(Box::new(self.get_post(retweeted_id).await?.ok_or(
                Error::DbError(format!(
                    "there's inconsistent data base status, cannot find post {}'s retweeted post {}",
                    post.id, retweeted_id
                )),
            )?))
        } else {
            None
        };
        let mut post: Post = post.try_into()?;
        post.retweeted_status = retweeted_status;
        post.user = user;
        Ok(post)
    }

    /// Hydrates a collection of internal posts in parallel.
    async fn hydrate_posts(&self, posts: Vec<PostInternal>) -> Vec<Post> {
        let posts = posts.into_iter().map(|p| {
            let id = p.id;
            self.hydrate_post(p).map_err(move |e| (id, e))
        });
        let (posts, errs): (Vec<_>, Vec<_>) = stream::iter(posts)
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .partition_result();

        warn!("{} posts constructed failed", errs.len());
        for (id, err) in errs {
            warn!("{id} cons failed: {err}");
        }
        posts
    }
}

#[async_trait]
impl Storage for StorageImpl {
    async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        user::get_user(&self.db_pool, uid).await
    }

    async fn get_users_by_ids(&self, ids: &[i64]) -> Result<Vec<User>> {
        user::get_users_by_ids(&self.db_pool, ids).await
    }

    async fn search_users_by_screen_name_prefix(&self, prefix: &str) -> Result<Vec<User>> {
        user::search_users_by_screen_name_prefix(&self.db_pool, prefix).await
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        self._save_post(post.clone()).await
    }

    async fn get_post(&self, id: i64) -> Result<Option<Post>> {
        self.get_post(id).await
    }

    async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPosts> {
        let (posts_internal, total_items) = post::query_posts(&self.db_pool, query).await?;
        let posts = self.hydrate_posts(posts_internal).await;
        Ok(PaginatedPosts { posts, total_items })
    }

    async fn query_all_post_ids(&self, query: PostQuery) -> Result<Vec<i64>> {
        post::query_all_post_ids(&self.db_pool, query).await
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        user::save_user(&self.db_pool, user).await
    }

    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        post::mark_post_unfavorited(&self.db_pool, id).await
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        post::mark_post_favorited(&self.db_pool, id).await
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        post::get_posts_id_to_unfavorite(&self.db_pool).await
    }

    async fn get_picture_blob(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<Option<Bytes>> {
        self.pic_storage
            .get_picture_blob(&ctx.config.picture_path, &self.db_pool, url)
            .await
    }

    async fn save_picture(&self, ctx: Arc<TaskContext>, picture: &Picture) -> Result<()> {
        self.pic_storage
            .save_picture(&ctx.config.picture_path, &self.db_pool, picture)
            .await
    }

    async fn picture_saved(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<bool> {
        self.pic_storage
            .picture_saved(&ctx.config.picture_path, &self.db_pool, url)
            .await
    }

    async fn get_video_blob(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<Option<Bytes>> {
        self.video_storage
            .get_video_blob(&ctx.config.video_path, &self.db_pool, url)
            .await
    }

    async fn save_video(&self, ctx: Arc<TaskContext>, video: &Video) -> Result<()> {
        self.video_storage
            .save_video(&ctx.config.video_path, &self.db_pool, video)
            .await
    }

    async fn video_saved(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<bool> {
        self.video_storage
            .video_saved(&ctx.config.video_path, &self.db_pool, url)
            .await
    }

    async fn get_picture_path(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<Option<PathBuf>> {
        let path = picture::get_picture_path(&self.db_pool, url).await?;
        Ok(path.map(|p| ctx.config.picture_path.join(p)))
    }

    async fn get_attached_infos(&self, post_id: i64) -> Result<Vec<PictureInfo>> {
        picture::get_pictures_by_post_id(&self.db_pool, post_id).await
    }

    async fn get_avatar_info(&self, user_id: i64) -> Result<Option<PictureInfo>> {
        picture::get_avatar_by_user_id(&self.db_pool, user_id).await
    }

    async fn get_avatar_infos(&self, user_id: i64) -> Result<Vec<PictureInfo>> {
        picture::get_avatars_by_user_id(&self.db_pool, user_id).await
    }

    async fn get_users_with_duplicate_avatars(&self) -> Result<Vec<i64>> {
        picture::get_users_with_duplicate_avatars(&self.db_pool).await
    }

    async fn get_pictures_by_ids(&self, ids: &[String]) -> Result<Vec<PictureInfo>> {
        picture::get_pictures_by_ids(&self.db_pool, ids).await
    }

    async fn get_pictures_by_id(&self, id: &str) -> Result<Vec<PictureInfo>> {
        picture::get_pictures_by_id(&self.db_pool, id).await
    }

    async fn get_duplicate_pic_ids(&self) -> Result<Vec<String>> {
        picture::get_duplicate_pic_ids(&self.db_pool).await
    }

    fn get_all_pictures(&self) -> impl Stream<Item = Result<PictureInfo>> + Send + '_ {
        const BATCH_SIZE: u64 = 100;
        let storage = self.clone();
        stream::unfold(
            (storage, 0u64, Vec::new(), false), // state：storage, offset, buffer, finished
            |(storage, offset, mut buffer, mut finished)| async move {
                if finished && buffer.is_empty() {
                    return None;
                }

                if buffer.is_empty() && !finished {
                    match picture::get_pictures_batch(&storage.db_pool, offset, BATCH_SIZE).await {
                        Ok(pics) => {
                            if pics.len() < BATCH_SIZE as usize {
                                finished = true;
                            }
                            if pics.is_empty() {
                                return None;
                            }
                            buffer = pics;
                        }
                        Err(e) => return Some((Err(e), (storage, offset, buffer, true))),
                    }
                }

                if !buffer.is_empty() {
                    let pic = buffer.remove(0);
                    let next_offset = if buffer.is_empty() {
                        offset + BATCH_SIZE
                    } else {
                        offset
                    };
                    Some((Ok(pic), (storage, next_offset, buffer, finished)))
                } else {
                    None
                }
            },
        )
    }

    async fn count_pictures(&self) -> Result<u64> {
        picture::count_pictures(&self.db_pool).await
    }

    async fn delete_picture(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<()> {
        self.pic_storage
            .delete_picture(&ctx.config.picture_path, &self.db_pool, url)
            .await
    }

    async fn delete_picture_by_url(&self, ctx: Arc<TaskContext>, url: &Url) -> Result<()> {
        let picture_path = &ctx.config.picture_path;
        // Get the relative path from database
        if let Some(relative_path) = picture::get_picture_path(&self.db_pool, url).await? {
            let absolute_path = picture_path.join(relative_path);
            // Try to delete the file, tolerate if it doesn't exist
            let _ = tokio::fs::remove_file(&absolute_path).await;
        }
        // Delete from database
        picture::delete_picture_by_url(&self.db_pool, url).await
    }

    async fn delete_post(&self, ctx: Arc<TaskContext>, id: i64) -> Result<()> {
        let picture_path = ctx.config.picture_path.clone();
        let video_path = ctx.config.video_path.clone();
        let Some(post) = post::get_post(&self.db_pool, id).await? else {
            return Ok(());
        };
        if post.retweeted_id.is_some() {
            self.pic_storage
                .delete_post_pictures(&picture_path, &self.db_pool, id)
                .await?;
            self.video_storage
                .delete_post_videos(&video_path, &self.db_pool, id)
                .await?;
            post::delete_post(&self.db_pool, id).await
        } else {
            let mut ids = post::get_retweet_ids(&self.db_pool, id).await?;
            ids.push(id);
            self.pic_storage
                .batch_delete_posts_pictures(&picture_path, &self.db_pool, &ids)
                .await?;
            self.video_storage
                .batch_delete_posts_videos(&video_path, &self.db_pool, &ids)
                .await?;
            post::batch_delete_posts(&self.db_pool, &ids).await
        }
    }

    async fn get_invalid_posts_ids(&self, clean_retweeted_invalid: bool) -> Result<Vec<i64>> {
        post::get_invalid_posts_ids(&self.db_pool, clean_retweeted_invalid).await
    }
}

#[cfg(test)]
mod local_tests {
    use std::{
        collections::{HashMap, HashSet},
        path::Path,
    };

    use futures::TryStreamExt;
    use itertools::Itertools;
    use tempfile::TempDir;
    use tokio::fs::read_to_string;

    use super::*;
    use crate::{
        api::{favorites::FavoritesSucc, profile_statuses::ProfileStatusesSucc},
        config::Config,
        core::task_manager::{TaskManager, TaskType},
        models::{PictureDefinition, Post, VideoMeta},
    };

    async fn setup_storage() -> StorageImpl {
        let db_pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&db_pool).await.unwrap();
        StorageImpl {
            db_pool,
            pic_storage: Default::default(),
            video_storage: Default::default(),
        }
    }

    async fn create_test_posts() -> Vec<Post> {
        let favorites = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json");
        let s = read_to_string(favorites).await.unwrap();
        let favs = serde_json::from_str::<FavoritesSucc>(s.as_str()).unwrap();
        let mut favs: Vec<Post> = favs
            .favorites
            .into_iter()
            .map(|p| p.status.try_into())
            .collect::<Result<_>>()
            .unwrap();
        let profile_statuses =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/profile_statuses.json");
        let statuses = serde_json::from_str::<ProfileStatusesSucc>(
            read_to_string(profile_statuses).await.unwrap().as_str(),
        )
        .unwrap();
        let statuses: Vec<Post> = statuses
            .cards
            .into_iter()
            .filter_map(|c| c.mblog.map(|p| p.try_into()))
            .collect::<Result<_>>()
            .unwrap();
        favs.extend(statuses);
        favs
    }

    async fn setup_task_context() -> (Arc<TaskContext>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let picture_path = temp_dir.path().join("pictures");
        let video_path = temp_dir.path().join("videos");

        let config = Config {
            picture_path,
            video_path,
            ..Default::default()
        };

        let task_manager = Arc::new(TaskManager::new());
        task_manager
            .start_task(0, TaskType::Export, "test".into(), 0)
            .unwrap();

        let ctx = Arc::new(TaskContext {
            task_id: Some(0),
            config,
            task_manager,
        });
        (ctx, temp_dir)
    }

    fn create_test_picture(post_id: i64, name: &str) -> Picture {
        let url = format!("http://example.com/pic_{name}.jpg");
        Picture {
            meta: PictureMeta::attached(&url, post_id, PictureDefinition::Large).unwrap(),
            blob: Bytes::from_static(b"test_image_data"),
        }
    }

    fn create_test_video(post_id: i64) -> Video {
        let url = format!(
            "https://video.weibo.com/media/play?livephoto=https%3A%2F%2Fus.sinaimg.cn%2Fvideo_{}.mov",
            post_id
        );
        Video {
            meta: VideoMeta::new(&url, post_id).unwrap(),
            blob: Bytes::from_static(b"test_video_data"),
        }
    }

    async fn create_test_users() -> Vec<User> {
        create_test_posts()
            .await
            .into_iter()
            .filter_map(|p| p.user)
            .unique_by(|u| u.id)
            .collect()
    }

    #[tokio::test]
    async fn test_user_functions() {
        let storage = setup_storage().await;
        let users = create_test_users().await;
        for user in users.iter() {
            storage.save_user(user).await.unwrap();
        }

        // Test get_users_by_ids
        let user_ids: Vec<i64> = users.iter().map(|u| u.id).collect();
        let fetched_users = storage.get_users_by_ids(&user_ids).await.unwrap();
        assert_eq!(fetched_users.len(), users.len());
        assert_eq!(
            fetched_users.iter().map(|u| u.id).collect::<HashSet<_>>(),
            users.iter().map(|u| u.id).collect::<HashSet<_>>()
        );

        // Test search_users_by_screen_name_prefix
        let Some(first_user) = users.first() else {
            return;
        };
        let prefix = &first_user.screen_name[0..3];
        let searched_users = storage
            .search_users_by_screen_name_prefix(prefix)
            .await
            .unwrap();
        assert!(!searched_users.is_empty());
        for user in searched_users {
            assert!(
                user.screen_name
                    .to_lowercase()
                    .starts_with(&prefix.to_lowercase())
            );
        }
    }

    #[tokio::test]
    async fn test_save_and_get_favorites() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;

        let mut favorited_sum = 0;
        let posts = posts
            .into_iter()
            .map(|p| (p.id, p))
            .collect::<HashMap<i64, Post>>();
        for post in posts.values() {
            if post.favorited {
                favorited_sum += 1;
            }
            storage.save_post(post).await.unwrap();
        }

        let query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            search_term: None,
            is_favorited: true,
            reverse_order: false,
            page: 1,
            posts_per_page: 1_000_000_000,
        };
        let paginated_posts = storage.query_posts(query).await.unwrap();
        let fetched_posts = paginated_posts.posts;

        assert_eq!(fetched_posts.len(), favorited_sum);

        for fetched in fetched_posts.iter() {
            let original = posts.get(&fetched.id).unwrap();
            if let (Some(original_user), Some(fetched_user)) = (&original.user, &fetched.user) {
                assert_eq!(original_user.id, fetched_user.id);
            }
            if let (Some(original_retweeted), Some(fetched_retweeted)) =
                (&original.retweeted_status, &fetched.retweeted_status)
            {
                assert_eq!(original_retweeted.id, fetched_retweeted.id);
                if let (Some(original_retweeted_user), Some(fetched_retweeted_user)) =
                    (&original_retweeted.user, &fetched_retweeted.user)
                {
                    assert_eq!(original_retweeted_user.id, fetched_retweeted_user.id);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_get_posts() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;
        let mut ids = HashSet::new();
        for post in posts.iter() {
            if let Some(ret) = post.retweeted_status.as_ref() {
                ids.insert(ret.id);
            }
            ids.insert(post.id);
            storage.save_post(post).await.unwrap();
        }

        let mut query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            search_term: None,
            is_favorited: false,
            reverse_order: false,
            page: 1,
            posts_per_page: 1_000_000,
        };
        let fetched_posts = storage.query_posts(query.clone()).await.unwrap();
        assert_eq!(fetched_posts.posts.len(), ids.len());

        query.reverse_order = true;
        let fetched_posts_rev = storage.query_posts(query).await.unwrap();
        assert_eq!(fetched_posts_rev.posts.len(), ids.len());
    }

    #[tokio::test]
    async fn test_get_ones_posts() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;
        let uid = 1786055427;
        let ones_posts_num = posts
            .iter()
            .filter(|p| {
                p.user.is_some() && p.user.as_ref().unwrap().id == uid
                    || p.retweeted_status.is_some()
                        && p.retweeted_status.as_ref().unwrap().id == uid
            })
            .count();

        for post in posts.iter() {
            storage.save_post(post).await.unwrap();
        }

        let mut query = PostQuery {
            user_id: Some(uid),
            start_date: None,
            end_date: None,
            search_term: None,
            is_favorited: false,
            reverse_order: false,
            page: 1,
            posts_per_page: ones_posts_num as u32,
        };
        let fetched_posts = storage.query_posts(query.clone()).await.unwrap();
        assert_eq!(fetched_posts.posts.len(), ones_posts_num);

        query.reverse_order = true;
        let fetched_posts_rev = storage.query_posts(query).await.unwrap();
        assert_eq!(fetched_posts_rev.posts.len(), ones_posts_num);
    }

    #[tokio::test]
    async fn test_favorites_logic() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;

        let mut favorited = 0;
        let mut not_favorited = vec![];
        for post in posts {
            if post.favorited {
                favorited += 1;
            } else {
                not_favorited.push(post.id);
            }
            storage.save_post(&post).await.unwrap();
        }

        let query = PostQuery {
            user_id: None,
            start_date: None,
            end_date: None,
            search_term: None,
            is_favorited: true,
            reverse_order: false,
            page: 1,
            posts_per_page: 2,
        };
        let paginated_posts = storage.query_posts(query).await.unwrap();
        assert_eq!(paginated_posts.total_items, favorited);

        let to_unfav = storage.get_posts_id_to_unfavorite().await.unwrap();
        assert_eq!(to_unfav.len(), favorited as usize);

        for id in to_unfav.iter().take(to_unfav.len() / 3) {
            storage.mark_post_unfavorited(*id).await.unwrap();
        }

        assert_eq!(
            storage.get_posts_id_to_unfavorite().await.unwrap().len() as u64,
            favorited - favorited / 3
        );

        for id in not_favorited.iter().take(to_unfav.len() / 3) {
            storage.mark_post_favorited(*id).await.unwrap();
        }

        assert_eq!(
            storage.get_posts_id_to_unfavorite().await.unwrap().len() as u64,
            favorited - favorited / 3 + not_favorited.len() as u64 / 3
        );
    }

    #[tokio::test]
    async fn test_media_and_delete_post() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await; // _temp_dir keeps the dir alive

        let mut posts = create_test_posts().await;
        let post = posts.remove(0);
        storage.save_post(&post).await.unwrap();

        let picture = create_test_picture(post.id, "post_pic");
        let video = create_test_video(post.id);

        // Test save_picture and picture_saved
        storage.save_picture(ctx.clone(), &picture).await.unwrap();
        assert!(
            storage
                .picture_saved(ctx.clone(), picture.meta.url())
                .await
                .unwrap()
        );

        // Test save_video and video_saved
        storage.save_video(ctx.clone(), &video).await.unwrap();
        assert!(
            storage
                .video_saved(ctx.clone(), video.meta.url())
                .await
                .unwrap()
        );

        // Test get_picture_blob
        let blob = storage
            .get_picture_blob(ctx.clone(), picture.meta.url())
            .await
            .unwrap();
        assert_eq!(blob.as_deref(), Some(&b"test_image_data"[..]));

        // Test get_video_blob
        let blob = storage
            .get_video_blob(ctx.clone(), video.meta.url())
            .await
            .unwrap();
        assert_eq!(blob.as_deref(), Some(&b"test_video_data"[..]));

        // Test get_picture_path
        let path = storage
            .get_picture_path(ctx.clone(), picture.meta.url())
            .await
            .unwrap();
        assert!(path.is_some());
        assert!(path.unwrap().exists());

        // Test get_attached_infos
        let infos = storage.get_attached_infos(post.id).await.unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].meta.url(), picture.meta.url());

        // Test delete_post
        storage.delete_post(ctx.clone(), post.id).await.unwrap();
        assert!(storage.get_post(post.id).await.unwrap().is_none());
        // Check if picture is also deleted
        assert!(
            !storage
                .picture_saved(ctx.clone(), picture.meta.url())
                .await
                .unwrap()
        );
        // Check if video is also deleted
        assert!(
            !storage
                .video_saved(ctx.clone(), video.meta.url())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_picture_queries_and_delete() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await;
        let users = create_test_users().await;
        let user1 = users.first().unwrap();
        storage.save_user(user1).await.unwrap();

        // Setup Avatar
        let avatar_pic = Picture {
            meta: PictureMeta::Avatar {
                url: Url::parse("http://example.com/avatar.jpg").unwrap(),
                user_id: user1.id,
            },
            blob: Bytes::from_static(b"avatar_data"),
        };
        storage
            .save_picture(ctx.clone(), &avatar_pic)
            .await
            .unwrap();

        // Test get_avatar_info and get_avatar_infos
        let avatar_info = storage.get_avatar_info(user1.id).await.unwrap();
        assert!(avatar_info.is_some());
        assert_eq!(avatar_info.unwrap().meta.url(), avatar_pic.meta.url());

        let avatar_infos = storage.get_avatar_infos(user1.id).await.unwrap();
        assert_eq!(avatar_infos.len(), 1);

        // Test get_users_with_duplicate_avatars
        let avatar_pic_2 = Picture {
            meta: PictureMeta::Avatar {
                url: Url::parse("http://example.com/avatar2.jpg").unwrap(),
                user_id: user1.id,
            },
            blob: Bytes::from_static(b"avatar_data_2"),
        };
        storage
            .save_picture(ctx.clone(), &avatar_pic_2)
            .await
            .unwrap();
        let duplicate_users = storage.get_users_with_duplicate_avatars().await.unwrap();
        assert_eq!(duplicate_users, vec![user1.id]);

        // Setup attached pictures with same ID
        let post_id = 12345;
        let pic1_id = "test_pic_id".to_string();
        let pic1_url = format!("http://example.com/large/{}.jpg", pic1_id);
        let pic1 = Picture {
            meta: PictureMeta::attached(&pic1_url, post_id, PictureDefinition::Large).unwrap(),
            blob: Bytes::from_static(b"data1"),
        };
        let pic2_url = format!("http://example.com/thumb/{}.jpg", pic1_id);
        let pic2 = Picture {
            meta: PictureMeta::attached(&pic2_url, post_id, PictureDefinition::Thumbnail).unwrap(),
            blob: Bytes::from_static(b"data2"),
        };
        storage.save_picture(ctx.clone(), &pic1).await.unwrap();
        storage.save_picture(ctx.clone(), &pic2).await.unwrap();

        // Test get_pictures_by_id (singular id, plural results)
        let pic_infos = storage.get_pictures_by_id(&pic1_id).await.unwrap();
        assert_eq!(pic_infos.len(), 2);

        // Test get_pictures_by_ids (plural ids)
        let pic_infos = storage
            .get_pictures_by_ids(std::slice::from_ref(&pic1_id))
            .await
            .unwrap();
        assert_eq!(pic_infos.len(), 2);

        // Test get_duplicate_pic_ids
        let dup_ids = storage.get_duplicate_pic_ids().await.unwrap();
        assert!(dup_ids.contains(&pic1_id));

        // Test delete_picture
        storage
            .delete_picture(ctx.clone(), pic1.meta.url())
            .await
            .unwrap();
        assert!(
            !storage
                .picture_saved(ctx.clone(), pic1.meta.url())
                .await
                .unwrap()
        );
        // The other pic with the same id should still be there
        assert!(
            storage
                .picture_saved(ctx.clone(), pic2.meta.url())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_all_pictures_empty() {
        let storage = setup_storage().await;

        // Collect all pictures from empty database
        let pictures: Vec<_> = storage.get_all_pictures().try_collect().await.unwrap();

        assert!(pictures.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_pictures_multiple_batches() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await;

        // Save more pictures than BATCH_SIZE (100) to test batching
        let post_id = 1002i64;
        let num_pictures = 250;
        for i in 0..num_pictures {
            let url = format!("http://example.com/batch_pic_{}.jpg", i);
            let picture = Picture {
                meta: PictureMeta::attached(&url, post_id, PictureDefinition::Large).unwrap(),
                blob: Bytes::from(format!("batch_image_data_{}", i)),
            };
            storage.save_picture(ctx.clone(), &picture).await.unwrap();
        }

        // Test get_all_pictures returns all pictures
        let pictures: Vec<_> = storage
            .get_all_pictures()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(pictures.len(), num_pictures);
    }

    #[tokio::test]
    async fn test_get_all_pictures_with_avatars() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await;

        // Create a user with avatar
        let users = create_test_users().await;
        let user = users.first().unwrap();
        storage.save_user(user).await.unwrap();

        // Save avatar
        let avatar = Picture {
            meta: PictureMeta::Avatar {
                url: Url::parse("http://example.com/avatar_test.jpg").unwrap(),
                user_id: user.id,
            },
            blob: Bytes::from_static(b"avatar_data"),
        };
        storage.save_picture(ctx.clone(), &avatar).await.unwrap();

        // Save some attached pictures
        let post_id = 1003i64;
        let pic = Picture {
            meta: PictureMeta::attached(
                "http://example.com/attached.jpg",
                post_id,
                PictureDefinition::Large,
            )
            .unwrap(),
            blob: Bytes::from_static(b"attached_data"),
        };
        storage.save_picture(ctx.clone(), &pic).await.unwrap();

        // Test get_all_pictures returns both avatar and attached pictures
        let pictures: Vec<_> = storage.get_all_pictures().try_collect().await.unwrap();

        // Should have 1 avatar + 1 attached = 2 pictures
        assert_eq!(pictures.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_picture_by_url_exists() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await;

        // Save a picture
        let url = "http://example.com/delete_test.jpg";
        let picture = Picture {
            meta: PictureMeta::attached(url, 9999, PictureDefinition::Large).unwrap(),
            blob: Bytes::from_static(b"test_image_data"),
        };
        storage.save_picture(ctx.clone(), &picture).await.unwrap();

        // Verify it exists
        assert!(
            storage
                .picture_saved(ctx.clone(), picture.meta.url())
                .await
                .unwrap()
        );

        // Delete the picture
        storage
            .delete_picture_by_url(ctx.clone(), picture.meta.url())
            .await
            .unwrap();

        // Verify it's deleted
        assert!(
            !storage
                .picture_saved(ctx.clone(), picture.meta.url())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_delete_picture_by_url_file_not_exists() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await;

        // Save a picture
        let url = "http://example.com/delete_test_no_file.jpg";
        let picture = Picture {
            meta: PictureMeta::attached(url, 9998, PictureDefinition::Large).unwrap(),
            blob: Bytes::from_static(b"test_image_data"),
        };
        storage.save_picture(ctx.clone(), &picture).await.unwrap();
        assert!(
            storage
                .picture_saved(ctx.clone(), picture.meta.url())
                .await
                .unwrap()
        );

        // Manually delete the file from filesystem (but keep database record)
        let path = storage
            .get_picture_path(ctx.clone(), picture.meta.url())
            .await
            .unwrap();
        if let Some(p) = path {
            tokio::fs::remove_file(&p).await.unwrap();
            // Verify file is gone but database record exists
            assert!(!p.exists());
        }

        // Delete via delete_picture_by_url (should tolerate missing file)
        storage
            .delete_picture_by_url(ctx.clone(), picture.meta.url())
            .await
            .unwrap();

        // Verify database record is deleted
        assert!(
            !storage
                .picture_saved(ctx.clone(), picture.meta.url())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_delete_picture_by_url_not_exists() {
        let storage = setup_storage().await;
        let (ctx, _temp_dir) = setup_task_context().await;

        // Try to delete a picture that doesn't exist
        let url = Url::parse("http://example.com/nonexistent.jpg").unwrap();

        // Should not panic, just succeed
        storage
            .delete_picture_by_url(ctx.clone(), &url)
            .await
            .unwrap();
    }
}
