mod picture_client;
mod post_client;
mod user_client;

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::{debug, info, trace};
use serde_json::to_string;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

use super::app::models::{Picture, Post, User};
use crate::app::Storage;

const VALIDE_DB_VERSION: i64 = 2;
const DATABASE: &str = "res/weiback.db";

#[derive(Debug, Clone)]
pub struct StorageImpl {
    db_path: PathBuf,
    db_pool: Option<SqlitePool>,
}

impl StorageImpl {
    pub fn new() -> Self {
        StorageImpl {
            db_path: std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join(DATABASE),
            db_pool: None,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        debug!("initing...");
        if self.db_path.is_file() {
            info!("db {:?} exists", self.db_path);
            self.db_pool = Some(SqlitePool::connect(self.db_path.to_str().unwrap()).await?);
            self.check_db_version().await?;
        } else {
            info!("db {:?} not exists, create it", self.db_path);
            if !self.db_path.parent().unwrap().exists() {
                let mut dir_builder = tokio::fs::DirBuilder::new();
                dir_builder.recursive(true);
                dir_builder
                    .create(
                        self.db_path
                            .parent()
                            .ok_or(anyhow!("{:?} should have parent", self.db_path))?,
                    )
                    .await?;
            } else if self.db_path.parent().unwrap().is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "export folder is a already exist file",
                )
                .into());
            }
            Sqlite::create_database(self.db_path.to_str().unwrap()).await?;
            self.db_pool = Some(SqlitePool::connect(self.db_path.to_str().unwrap()).await?);
            self.create_db().await?;
        }
        Ok(())
    }

    async fn check_db_version(&mut self) -> Result<()> {
        let version = sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
            .fetch_one(self.db().unwrap())
            .await?;
        debug!("db version: {}", version.0);
        if version.0 == VALIDE_DB_VERSION {
            Ok(())
        } else {
            Err(anyhow!("Invalid database version, please upgrade db file"))
        }
    }

    async fn create_db(&mut self) -> Result<()> {
        let mut conn = self.db_pool.as_ref().unwrap().acquire().await?;
        Post::create_table(conn.as_mut()).await?;
        User::create_table(conn.as_mut()).await?;
        Picture::create_table(conn).await?;
        sqlx::query(format!("PRAGMA user_version = {};", VALIDE_DB_VERSION).as_str())
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    pub fn db(&self) -> Option<&SqlitePool> {
        self.db_pool.as_ref()
    }

    async fn _get_user(&self, id: i64) -> Result<Option<User>> {
        let user = sqlx::query_as::<Sqlite, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(user)
    }

    async fn _save_user(&self, user: &User) -> Result<()> {
        debug!("insert user: {}", user.id);
        trace!("insert user: {:?}", user);
        let result = sqlx::query(
            "INSERT OR IGNORE INTO users (\
             id,\
             profile_url,\
             screen_name,\
             profile_image_url,\
             avatar_large,\
             avatar_hd,\
             planet_video,\
             v_plus,\
             pc_new,\
             verified,\
             verified_type,\
             domain,\
             weihao,\
             verified_type_ext,\
             follow_me,\
             following,\
             mbrank,\
             mbtype,\
             icon_list,\
             backedup)\
             VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id)
        .bind(&user.profile_url)
        .bind(&user.screen_name)
        .bind(&user.profile_image_url)
        .bind(&user.avatar_large)
        .bind(&user.avatar_hd)
        .bind(user.planet_video)
        .bind(user.v_plus)
        .bind(user.pc_new)
        .bind(user.verified)
        .bind(user.verified_type)
        .bind(&user.domain)
        .bind(&user.weihao)
        .bind(user.verified_type_ext)
        .bind(user.follow_me)
        .bind(user.following)
        .bind(user.mbrank)
        .bind(user.mbtype)
        .bind(user.icon_list.as_ref().and_then(|v| to_string(&v).ok()))
        .bind(false)
        .execute(self.db_pool.as_ref().unwrap())
        .await?;
        trace!("insert user {user:?}, result {result:?}");
        Ok(())
    }

    async fn _get_post(&self, id: i64) -> Result<Option<Post>> {
        if let Some(mut post) = sqlx::query_as::<Sqlite, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(id)
            .fetch_optional(self.db_pool.as_ref().unwrap())
            .await?
        {
            if let Some(uid) = post.uid {
                post.user = self._get_user(uid).await?;
            }
            return Ok(Some(post));
        }
        Ok(None)
    }

    async fn load_complete_post(&self, post: &mut Post) -> Result<()> {
        if let Some(uid) = post.uid {
            post.user = self._get_user(uid).await?;
        }
        if let Some(retweeted_id) = post.retweeted_id {
            post.retweeted_status = Some(Box::new(self._get_post(retweeted_id).await?.ok_or(
                anyhow!(
                    "cannot find retweeted post {} of post {}",
                    retweeted_id,
                    post.id
                ),
            )?));
        }
        Ok(())
    }

    async fn _save_post(&self, post: &Post, overwrite: bool) -> Result<()> {
        if let Some(user) = post.user.as_ref() {
            self._save_user(user).await?;
        }
        sqlx::query(
            format!(
                "INSERT OR {} INTO posts (\
             id,\
             mblogid,\
             text_raw,\
             source,\
             region_name,\
             deleted,\
             uid,\
             pic_ids,\
             pic_num,\
             retweeted_id,\
             url_struct,\
             topic_struct,\
             tag_struct,\
             tags,\
             customIcons,\
             number_display_strategy,\
             mix_media_info,\
             visible,\
             text,\
             attitudes_status,\
             showFeedRepost,\
             showFeedComment,\
             pictureViewerSign,\
             showPictureViewer,\
             favorited,\
             can_edit,\
             is_paid,\
             share_repost_type,\
             rid,\
             pic_infos,\
             cardid,\
             pic_bg_new,\
             mark,\
             mblog_vip_type,\
             reposts_count,\
             comments_count,\
             attitudes_count,\
             mlevel,\
             complaint,\
             content_auth,\
             is_show_bulletin,\
             repost_type,\
             edit_count,\
             mblogtype,\
             textLength,\
             isLongText,\
             rcList,\
             annotations,\
             geo,\
             pic_focus_point,\
             page_info,\
             title,\
             continue_tag,\
             comment_manage_info,\
             client_only,\
             unfavorited,\
             created_at,\
             created_at_timestamp,\
             created_at_tz)\
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                if overwrite { "REPLACE" } else { "IGNORE" }
            )
            .as_str(),
        )
        .bind(post.id)
        .bind(&post.mblogid)
        .bind(&post.text_raw)
        .bind(&post.source)
        .bind(&post.region_name)
        .bind(post.deleted)
        .bind(post.uid)
        .bind(&post.pic_ids)
        .bind(post.pic_num)
        .bind(post.retweeted_id)
        .bind(&post.url_struct)
        .bind(&post.topic_struct)
        .bind(&post.tag_struct)
        .bind(&post.tags)
        .bind(&post.custom_icons)
        .bind(&post.number_display_strategy)
        .bind(&post.mix_media_info)
        .bind(&post.visible)
        .bind(&post.text)
        .bind(post.attitudes_status)
        .bind(post.show_feed_repost)
        .bind(post.show_feed_comment)
        .bind(post.picture_viewer_sign)
        .bind(post.show_picture_viewer)
        .bind(post.favorited)
        .bind(post.can_edit)
        .bind(post.is_paid)
        .bind(post.share_repost_type)
        .bind(&post.rid)
        .bind(&post.pic_infos)
        .bind(&post.cardid)
        .bind(&post.pic_bg_new)
        .bind(&post.mark)
        .bind(post.mblog_vip_type)
        .bind(post.reposts_count)
        .bind(post.comments_count)
        .bind(post.attitudes_count)
        .bind(post.mlevel)
        .bind(&post.complaint)
        .bind(post.content_auth)
        .bind(post.is_show_bulletin)
        .bind(post.repost_type)
        .bind(post.edit_count)
        .bind(post.mblogtype)
        .bind(post.text_length)
        .bind(post.is_long_text)
        .bind(&post.rc_list)
        .bind(&post.annotations)
        .bind(&post.geo)
        .bind(&post.pic_focus_point)
        .bind(&post.page_info)
        .bind(&post.title)
        .bind(&post.continue_tag)
        .bind(&post.comment_manage_info)
        .bind(post.client_only)
        .bind(post.unfavorited)
        .bind(&post.created_at)
        .bind(post.created_at_timestamp)
        .bind(&post.created_at_tz)
        .execute(self.db_pool.as_ref().unwrap())
        .await?;
        Ok(())
    }
}

impl Storage for StorageImpl {
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        debug!("unfav post {} in db", id);
        sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
            .bind(id)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        debug!("mark favorited post {} in db", id);
        sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
            .bind(id)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        Ok(
            sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
                .fetch_one(self.db_pool.as_ref().unwrap())
                .await?
                .0,
        )
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        debug!("query all posts to unfavorite");
        Ok(sqlx::query_as::<Sqlite, (i64,)>(
            "SELECT id FROM posts WHERE unfavorited == false and favorited;",
        )
        .fetch_all(self.db_pool.as_ref().unwrap())
        .await?
        .into_iter()
        .map(|t| t.0)
        .collect())
    }

    async fn get_post(&self, id: i64) -> Result<Option<Post>> {
        debug!("query post, id: {id}");
        if let Some(mut post) = self._get_post(id).await? {
            if let Some(retweeted_id) = post.retweeted_id {
                post.retweeted_status = Some(Box::new(self._get_post(retweeted_id).await?.ok_or(
                    anyhow!("cannot find retweeted post {} of post {}", retweeted_id, id),
                )?));
            }
            Ok(Some(post))
        } else {
            Ok(None)
        }
    }

    async fn get_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Vec<Post>> {
        debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
        let sql_expr = if reverse {
            "SELECT * FROM posts WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
        } else {
            "SELECT * FROM posts WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
        };
        let mut posts = sqlx::query_as::<sqlx::Sqlite, Post>(sql_expr)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db_pool.as_ref().unwrap())
            .await?;
        for post in posts.iter_mut() {
            self.load_complete_post(post).await?;
        }
        debug!("geted {} post from local", posts.len());
        Ok(posts)
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        debug!("insert post: {}", post.id);
        trace!("insert post: {:?}", post);
        self._save_post(post, true).await?;
        Ok(())
    }

    async fn get_user(&self, id: i64) -> Result<Option<User>> {
        self._get_user(id).await
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        self._save_user(&user).await
    }
}
