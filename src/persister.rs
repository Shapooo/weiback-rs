use std::path::PathBuf;

use anyhow;
use bytes::Bytes;
use log::{info, trace};
use sqlx::{
    migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Connection, Sqlite, SqliteConnection,
    SqlitePool,
};

use crate::sql_data::{PictureBlob, SqlPost, SqlUser};

const DATABASE_CREATE_SQL: &str = "CREATE TABLE IF NOT EXISTS fav_post(id INTEGER PRIMARY KEY, created_at VARCHAR, mblogid VARCHAR, text_raw TEXT, source VARCHAR, region_name VARCHAR, deleted BOOLEAN, uid INTEGER, pic_ids VARCHAR, pic_num INTEGER, retweeted_status INTEGER, url_struct json, topic_struct json, tag_struct json, number_display_strategy json, mix_media_info json, visible json, text TEXT, attitudes_status INTEGER, showFeedRepost BOOLEAN, showFeedComment BOOLEAN, pictureViewerSign BOOLEAN, showPictureViewer BOOLEAN, favorited BOOLEAN, can_edit BOOLEAN, is_paid BOOLEAN, share_repost_type INTEGER, rid VARCHAR, pic_infos VARCHAR, cardid VARCHAR, pic_bg_new VARCHAR, mark VARCHAR, mblog_vip_type INTEGER, reposts_count INTEGER, comments_count INTEGER, attitudes_count INTEGER, mlevel INTEGER, content_auth INTEGER, is_show_bulletin INTEGER, repost_type INTEGER, edit_count INTEGER, mblogtype INTEGER, text_length INTEGER, isLongText BOOLEAN, annotations json, geo json, pic_focus_point json, page_info json, title json, continue_tag json, comment_manage_info json); CREATE TABLE IF NOT EXISTS user(id INTEGER PRIMARY KEY, profile_url VARCHAR, screen_name VARCHAR, profile_image_url VARCHAR, avatar_large VARCHAR, avatar_hd VARCHAR, planet_video BOOLEAN, v_plus INTEGER, pc_new INTEGER, verified BOOLEAN, verified_type INTEGER, domain VARCHAR, weihao VARCHAR, verified_type_ext INTEGER, follow_me BOOLEAN, following BOOLEAN, mbrank INTEGER, mbtype INTEGER, icon_list VARCHAR); CREATE TABLE IF NOT EXISTS picture_blob(url VARCHAR PRIMARY KEY, id VARCHAR, blob BLOB);";

#[derive(Debug)]
pub struct Persister {
    db_pool: SqlitePool,
}

impl Persister {
    pub async fn build<P>(db: P) -> anyhow::Result<Self>
    where
        P: AsRef<str>,
    {
        let url = String::from("sqlite:") + db.as_ref();
        let db = PathBuf::from("db");
        if db.is_file() {
            info!("db file {:?} exists", db);
        } else {
            info!("db file {:?} not exists, create it", db);
            Sqlite::create_database(&url).await?;
            let mut db = SqliteConnection::connect(&url).await?;
            use futures::stream::StreamExt;
            sqlx::query(DATABASE_CREATE_SQL)
                .execute_many(&mut db)
                .await
                .for_each_concurrent(None, |res| async move {
                    let query_result = res.unwrap();
                    trace!(
                        "rows_affected {}, last_insert_rowid {}",
                        query_result.rows_affected(),
                        query_result.last_insert_rowid()
                    );
                })
                .await;
        }
        let pool = SqlitePoolOptions::new()
            .min_connections(2)
            .connect_lazy(&url)?;
        Ok(Persister { db_pool: pool })
    }

    pub async fn insert_post(&self, post: &SqlPost) -> anyhow::Result<()> {
        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO
fav_post VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&post.id)
        .bind(&post.created_at)
        .bind(&post.mblogid)
        .bind(&post.text_raw)
        .bind(&post.source)
        .bind(&post.region_name)
        .bind(&post.deleted)
        .bind(&post.uid)
        .bind(&post.pic_ids)
        .bind(&post.pic_num)
        .bind(&post.retweeted_status)
        .bind(&post.url_struct)
        .bind(&post.topic_struct)
        .bind(&post.tag_struct)
        .bind(&post.number_display_strategy)
        .bind(&post.mix_media_info)
        .bind(&post.visible)
        .bind(&post.text)
        .bind(&post.attitudes_status)
        .bind(&post.show_feed_repost)
        .bind(&post.show_feed_comment)
        .bind(&post.picture_viewer_sign)
        .bind(&post.show_picture_viewer)
        .bind(&post.favorited)
        .bind(&post.can_edit)
        .bind(&post.is_paid)
        .bind(&post.share_repost_type)
        .bind(&post.rid)
        .bind(&post.pic_infos)
        .bind(&post.cardid)
        .bind(&post.pic_bg_new)
        .bind(&post.mark)
        .bind(&post.mblog_vip_type)
        .bind(&post.reposts_count)
        .bind(&post.comments_count)
        .bind(&post.attitudes_count)
        .bind(&post.mlevel)
        .bind(&post.content_auth)
        .bind(&post.is_show_bulletin)
        .bind(&post.repost_type)
        .bind(&post.edit_count)
        .bind(&post.mblogtype)
        .bind(&post.text_length)
        .bind(&post.is_long_text)
        .bind(&post.annotations)
        .bind(&post.geo)
        .bind(&post.pic_focus_point)
        .bind(&post.page_info)
        .bind(&post.title)
        .bind(&post.continue_tag)
        .bind(&post.comment_manage_info)
        .execute(&self.db_pool)
        .await?;
        trace!("insert post {post:?} \nresult: {result:?}");
        Ok(())
    }

    pub async fn insert_img(&self, url: &str, id: &str, img: &[u8]) -> anyhow::Result<()> {
        let result = sqlx::query("INSERT OR IGNORE INTO picture_blob VALUES (?, ?, ?)")
            .bind(url)
            .bind(id)
            .bind(img)
            .execute(&self.db_pool)
            .await?;
        trace!("insert img {id}-{url}, result: {result:?}");
        Ok(())
    }

    pub async fn insert_user(&self, user: &SqlUser) -> anyhow::Result<()> {
        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO user VALUES
 (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&user.id)
        .bind(&user.profile_url)
        .bind(&user.screen_name)
        .bind(&user.profile_image_url)
        .bind(&user.avatar_large)
        .bind(&user.avatar_hd)
        .bind(&user.planet_video)
        .bind(&user.v_plus)
        .bind(&user.pc_new)
        .bind(&user.verified)
        .bind(&user.verified_type)
        .bind(&user.domain)
        .bind(&user.weihao)
        .bind(&user.verified_type_ext)
        .bind(&user.follow_me)
        .bind(&user.following)
        .bind(&user.mbrank)
        .bind(&user.mbtype)
        .bind(&user.icon_list)
        .execute(&self.db_pool)
        .await?;
        trace!("insert user {user:?}, result {result:?}");
        Ok(())
    }

    pub async fn query_img(&self, url: &str) -> anyhow::Result<Bytes> {
        let result: PictureBlob = sqlx::query_as("SELECT * FROM picture_blob WHERE url = ?")
            .bind(url)
            .fetch_one(&self.db_pool)
            .await?;
        Ok(result.blob.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::fetched_data::{FetchedPost, FetchedPosts};
    use crate::sql_data::{SqlPost, SqlUser};

    use super::*;
    use serde_json;
    use std::path::PathBuf;
    #[tokio::test]
    async fn post_build() {
        Persister::build("post_build.db").await.unwrap();
        std::fs::remove_file("post_build.db").unwrap();
    }

    #[tokio::test]
    async fn insert_post() {
        let pster = Persister::build("insert_post.db").await.unwrap();
        let txt = include_str!("../res/one.json");
        let post: SqlPost = SqlPost::from(serde_json::from_str::<FetchedPost>(txt).unwrap());
        pster.insert_post(&post).await.unwrap();
        std::fs::remove_file("insert_post.db").unwrap();
        std::fs::remove_file("insert_post.db-shm").unwrap();
        std::fs::remove_file("insert_post.db-wal").unwrap();
    }

    #[tokio::test]
    async fn insert_posts() {
        let pster = Persister::build("insert_posts.db").await.unwrap();
        let txt = include_str!("../res/full.json");
        let posts: Vec<SqlPost> = serde_json::from_str::<FetchedPosts>(txt)
            .unwrap()
            .data
            .into_iter()
            .map(SqlPost::from)
            .collect();
        for post in posts.iter() {
            pster.insert_post(post).await.unwrap();
        }
        std::fs::remove_file("insert_posts.db").unwrap();
        std::fs::remove_file("insert_posts.db-shm").unwrap();
        std::fs::remove_file("insert_posts.db-wal").unwrap();
    }

    #[tokio::test]
    async fn insert_users() {
        let pster = Persister::build("insert_users.db").await.unwrap();
        let txt = include_str!("../res/full.json");
        let users: Vec<SqlUser> = serde_json::from_str::<FetchedPosts>(txt)
            .unwrap()
            .data
            .into_iter()
            .filter_map(|p| p.user)
            .map(|user| SqlUser::from(user))
            .collect();
        for user in users.iter() {
            pster.insert_user(user).await.unwrap();
        }
        std::fs::remove_file("insert_users.db").unwrap();
        std::fs::remove_file("insert_users.db-shm").unwrap();
        std::fs::remove_file("insert_users.db-wal").unwrap();
    }

    #[tokio::test]
    async fn insert_img() {
        let img = include_bytes!("../res/example.jpg");
        let id = "example";
        let url = "https://test_url/example.jpg";
        let p = Persister::build("insert_img.db").await.unwrap();
        p.insert_img(url, id, img).await.unwrap();
        std::fs::write("./examp.jpg", p.query_img(url).await.unwrap()).unwrap();
        std::fs::remove_file("insert_img.db").unwrap();
        std::fs::remove_file("insert_img.db-shm").unwrap();
        std::fs::remove_file("insert_img.db-wal").unwrap();
    }
}
