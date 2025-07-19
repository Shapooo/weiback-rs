use std::pin::Pin;

use itertools::Itertools;
use log::debug;
use serde_json::to_string;
use sqlx::{Sqlite, SqlitePool};

use super::post_storage::PostStorage;
use super::user_storage::UserStorage;
use crate::error::{Error, Result};
use crate::models::picture::PictureMeta;
use crate::models::{Picture, Post, User};

#[derive(Debug, Clone)]
pub struct Processer {
    db_pool: SqlitePool,
}

impl Processer {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self { db_pool }
    }

    pub fn save_post(
        &self,
        mut post: Post,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            if let Some(user) = post.user {
                self.save_user(&user).await?;
            }
            if let Some(ret_post) = post.retweeted_status.take() {
                self.save_post(*ret_post).await?;
            }
            // self._save_post(&post.try_into()?, overwrite).await?;
            todo!();
        })
    }

    fn get_post(&self, id: i64) -> Pin<Box<dyn Future<Output = Result<Option<Post>>> + Send + '_>> {
        Box::pin(async move {
            let Some(post) = self._get_post(id).await? else {
                return Ok(None);
            };
            self.cons_compelete_post(post).await
        })
    }

    pub async fn cons_compelete_post(&self, post: PostStorage) -> Result<Option<Post>> {
        let user = if let Some(uid) = post.uid {
            self.get_user(uid).await?
        } else {
            None
        };

        let retweeted_status = if let Some(retweeted_id) = post.retweeted_id {
            Some(Box::new(self.get_post(retweeted_id).await?.ok_or(
                Error::Other(format!(
                    "cannot find retweeted post {} of post {}",
                    retweeted_id, post.id
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

    pub async fn get_user(&self, id: i64) -> Result<Option<User>> {
        let user = sqlx::query_as::<Sqlite, UserStorage>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db_pool)
            .await?;
        Ok(user.map(|u| u.into()))
    }

    pub async fn save_user(&self, user: &User) -> Result<()> {
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

    pub async fn get_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Vec<Post>> {
        debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
        let sql_expr = if reverse {
            "SELECT * FROM posts WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
        } else {
            "SELECT * FROM posts WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
        };
        let posts = sqlx::query_as::<Sqlite, PostStorage>(sql_expr)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db_pool)
            .await?;
        let (posts, _): (Vec<_>, Vec<_>) =
            futures::future::join_all(posts.into_iter().map(|p| self.cons_compelete_post(p)))
                .await
                .into_iter()
                .partition_result();
        let posts: Vec<_> = posts.into_iter().filter_map(|p| p).collect();
        // TODO: deal with err and none(s)
        debug!("geted {} post from local", posts.len());
        Ok(posts)
    }

    async fn _get_post(&self, id: i64) -> Result<Option<PostStorage>> {
        Ok(
            sqlx::query_as::<Sqlite, PostStorage>("SELECT * FROM posts WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.db_pool)
                .await?,
        )
    }

    async fn _save_post(&self, post: &PostStorage, overwrite: bool) -> Result<()> {
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
        .bind(to_string(&post.pic_infos)?)
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
