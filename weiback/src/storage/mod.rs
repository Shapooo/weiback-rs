#![allow(async_fn_in_trait)]
mod database;
mod internal;

use std::env::current_exe;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use itertools::Itertools;
use log::{debug, error, info};
use sqlx::{Sqlite, SqlitePool};
use tokio::runtime::Runtime;

use crate::config::get_config;
use crate::error::{Error, Result};
use crate::exporter::ExportOptions;
use crate::models::{Picture, Post, User};
use crate::utils::url_to_path;
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
    picture_path: PathBuf,
}

impl StorageImpl {
    pub fn new() -> Result<Self> {
        info!("Initializing storage...");
        let config = get_config();
        let config_read = config.read().map_err(|e| {
            error!("Failed to read config lock: {e}");
            e
        })?;
        let picture_path = config_read.picture_path.clone();
        drop(config_read);

        let db_pool = Runtime::new()?
            .block_on(database::create_db_pool())
            .map_err(|e| {
                error!("Failed to create database pool: {e}");
                e
            })?;

        let picture_path = current_exe()?.parent().unwrap().join(picture_path);
        info!("Storage initialized successfully.");
        Ok(StorageImpl {
            db_pool,
            picture_path,
        })
    }

    fn _save_post(&self, mut post: Post) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            debug!("Saving post with id: {}", post.id);
            let uid = post.user.as_ref().map(|u| u.id);
            if let Some(user) = post.user.take() {
                self._save_user(&user).await?;
            }
            let retweeted_id = post.retweeted_status.as_ref().map(|p| p.id);
            if let Some(ret_post) = post.retweeted_status.take() {
                self._save_post(*ret_post).await?;
            }
            let mut post_storage: PostInternal = post.try_into()?;
            post_storage.uid = uid;
            post_storage.retweeted_id = retweeted_id;
            match self.do_save_post_sql(&post_storage, true).await {
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
            let Some(post) = self.do_get_post_sql(id).await? else {
                return Ok(None);
            };
            self.hydrate_post(post).await
        })
    }

    async fn hydrate_post(&self, post: PostInternal) -> Result<Option<Post>> {
        let user = if let Some(uid) = post.uid {
            self._get_user(uid).await?
        } else {
            None
        };

        let retweeted_status = if let Some(retweeted_id) = post.retweeted_id {
            Some(Box::new(self.get_post(retweeted_id).await?.ok_or(
                Error::DbError(format!(
                    "there's inconsistent data base status, cannot find  post {}'s retweeted post {}",
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

    async fn _get_user(&self, id: i64) -> Result<Option<User>> {
        let user = sqlx::query_as::<Sqlite, UserInternal>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db_pool)
            .await?;
        Ok(user.map(|u| u.into()))
    }

    async fn _save_user(&self, user: &User) -> Result<()> {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO users (\
             id,\
             screen_name,\
             profile_image_url,\
             avatar_large,\
             avatar_hd,\
             verified,\
             verified_type,\
             domain,\
             follow_me,\
             following)\
             VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id)
        .bind(&user.screen_name)
        .bind(&user.profile_image_url)
        .bind(&user.avatar_large)
        .bind(&user.avatar_hd)
        .bind(user.verified)
        .bind(user.verified_type)
        .bind(&user.domain)
        .bind(user.follow_me)
        .bind(user.following)
        .bind(false)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    async fn _get_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Vec<Post>> {
        debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
        let sql_expr = if reverse {
            "SELECT * FROM posts WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
        } else {
            "SELECT * FROM posts WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
        };
        let posts = sqlx::query_as::<Sqlite, PostInternal>(sql_expr)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db_pool)
            .await?;
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

    async fn do_get_post_sql(&self, id: i64) -> Result<Option<PostInternal>> {
        Ok(
            sqlx::query_as::<Sqlite, PostInternal>("SELECT * FROM posts WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.db_pool)
                .await?,
        )
    }

    async fn do_save_post_sql(&self, post: &PostInternal, overwrite: bool) -> Result<()> {
        sqlx::query(
            format!(
                "INSERT OR {} INTO posts (\
                 id,\
                 mblogid,\
                 source,\
                 region_name,\
                 deleted,\
                 pic_ids,\
                 pic_num,\
                 url_struct,\
                 topic_struct,\
                 tag_struct,\
                 number_display_strategy,\
                 mix_media_info,\
                 text,\
                 attitudes_status,\
                 favorited,\
                 pic_infos,\
                 reposts_count,\
                 comments_count,\
                 attitudes_count,\
                 repost_type,\
                 edit_count,\
                 isLongText,\
                 geo,\
                 page_info,\
                 unfavorited,\
                 created_at,\
                 retweeted_id,\
                 uid)\
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                 ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                 ?, ?, ?, ?, ?, ?, ?, ?)",
                if overwrite { "REPLACE" } else { "IGNORE" }
            )
            .as_str(),
        )
        .bind(post.id)
        .bind(&post.mblogid)
        .bind(&post.source)
        .bind(&post.region_name)
        .bind(post.deleted)
        .bind(post.pic_num)
        .bind(&post.url_struct)
        .bind(&post.topic_struct)
        .bind(&post.tag_struct)
        .bind(&post.number_display_strategy)
        .bind(&post.mix_media_info)
        .bind(&post.text)
        .bind(post.attitudes_status)
        .bind(post.favorited)
        .bind(serde_json::to_string(&post.pic_infos)?)
        .bind(post.reposts_count)
        .bind(post.comments_count)
        .bind(post.attitudes_count)
        .bind(post.repost_type)
        .bind(post.edit_count)
        .bind(post.is_long_text)
        .bind(&post.geo)
        .bind(&post.page_info)
        .bind(post.unfavorited)
        .bind(&post.created_at)
        .bind(post.uid)
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }
}

impl Storage for Arc<StorageImpl> {
    async fn get_posts(&self, options: &ExportOptions) -> Result<Vec<Post>> {
        let start = options.range.start();
        let end = options.range.end();
        self._get_posts(*end - *start + 1, *start, options.reverse)
            .await
    }

    async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        self._get_user(uid).await
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        self._save_post(post.clone()).await
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        self._save_user(user).await
    }

    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        debug!("unfav post {id} in db");
        sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        debug!("mark favorited post {id} in db");
        sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        Ok(
            sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
                .fetch_one(&self.db_pool)
                .await?
                .0,
        )
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        debug!("query all posts to unfavorite");
        Ok(sqlx::query_as::<Sqlite, (i64,)>(
            "SELECT id FROM posts WHERE unfavorited == false and favorited;",
        )
        .fetch_all(&self.db_pool)
        .await?
        .into_iter()
        .map(|t| t.0)
        .collect())
    }

    // TODO: clarify semantic of Result and Option
    async fn get_picture_blob(&self, url: &str) -> Result<Option<Bytes>> {
        let path = url_to_path(url)?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap(); // promised to start with '/'
        let path = self.picture_path.join(relative_path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn save_picture(&self, picture: &Picture) -> Result<()> {
        let path = url_to_path(picture.meta.url())?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap(); // promised to start with '/'
        let path = self.picture_path.join(relative_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &picture.blob).await?;
        debug!("picture {} saved to {:?}", picture.meta.url(), path);
        Ok(())
    }
}
