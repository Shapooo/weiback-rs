#![allow(async_fn_in_trait)]
mod database;
mod internal;
mod picture_storage;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use itertools::Itertools;
use log::{debug, error, info};
use picture_storage::FileSystemPictureStorage;
use sqlx::SqlitePool;
use tokio::runtime::Runtime;

use crate::error::{Error, Result};
use crate::exporter::ExportOptions;
use crate::models::{Picture, Post, User};
use internal::post::{self, PostInternal};
use internal::user;

const VALIDE_DB_VERSION: i64 = 2;

pub trait Storage: Send + Sync + Clone + 'static {
    async fn save_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, uid: i64) -> Result<Option<User>>;
    async fn get_posts(&self, options: &ExportOptions) -> Result<Vec<Post>>;
    async fn save_post(&self, post: &Post) -> Result<()>;
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()>;
    async fn mark_post_favorited(&self, id: i64) -> Result<()>;
    async fn get_favorited_sum(&self) -> Result<u32>;
    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>>;
    fn save_picture(&self, picture: &Picture) -> impl Future<Output = Result<()>> + Send;
    async fn get_picture_blob(&self, url: &str) -> Result<Option<bytes::Bytes>>;
}

#[derive(Debug, Clone)]
pub struct StorageImpl {
    db_pool: SqlitePool,
    pic_storage: FileSystemPictureStorage,
}

impl StorageImpl {
    pub fn new() -> Result<Self> {
        info!("Initializing storage...");
        let db_pool = Runtime::new()?
            .block_on(database::create_db_pool())
            .map_err(|e| {
                error!("Failed to create database pool: {e}");
                e
            })?;
        let pic_storage = FileSystemPictureStorage::new()?;

        info!("Storage initialized successfully.");
        Ok(StorageImpl {
            db_pool,
            pic_storage,
        })
    }

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
            match post::save_post(&self.db_pool, &post_storage, true).await {
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

    fn get_post(&self, id: i64) -> Pin<Box<dyn Future<Output = Result<Option<Post>>> + Send + '_>> {
        Box::pin(async move {
            let Some(post) = post::get_post(&self.db_pool, id).await? else {
                return Ok(None);
            };
            self.hydrate_post(post).await
        })
    }

    async fn hydrate_post(&self, post: PostInternal) -> Result<Option<Post>> {
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
        Ok(Some(post))
    }
}

impl Storage for Arc<StorageImpl> {
    async fn get_posts(&self, options: &ExportOptions) -> Result<Vec<Post>> {
        let (start, end) = options.range.clone().into_inner();
        let posts = post::get_posts(&self.db_pool, end - start + 1, start, options.reverse).await?;
        let (posts, _): (Vec<_>, Vec<_>) =
            futures::future::join_all(posts.into_iter().map(|p| self.hydrate_post(p)))
                .await
                .into_iter()
                .partition_result();
        let posts: Vec<_> = posts.into_iter().flatten().collect();
        // TODO: deal with err and none(s)
        debug!("geted {} post from local", posts.len());
        Ok(posts)
    }

    async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        user::get_user(&self.db_pool, uid).await
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        self._save_post(post.clone()).await
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

    async fn get_favorited_sum(&self) -> Result<u32> {
        post::get_favorited_sum(&self.db_pool).await
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        post::get_posts_id_to_unfavorite(&self.db_pool).await
    }

    async fn get_picture_blob(&self, url: &str) -> Result<Option<Bytes>> {
        self.pic_storage.get_picture_blob(url).await
    }

    async fn save_picture(&self, picture: &Picture) -> Result<()> {
        self.pic_storage.save_picture(picture).await
    }
}
