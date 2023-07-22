use std::path::PathBuf;

use bytes::Bytes;
use futures::future::join_all;
use log::{debug, info, trace};
use serde::Serialize;
use serde_json::{from_str, to_value, Value};
use sqlx::FromRow;
use sqlx::{
    migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Connection, Sqlite, SqliteConnection,
    SqlitePool,
};

use crate::data::{Post, Posts, User};
use crate::error::{Error, Result};
use crate::utils::{pic_url_to_id, strip_url_queries};

const DATABASE_CREATE_SQL: &str = "CREATE TABLE IF NOT EXISTS posts(id INTEGER PRIMARY KEY, \
                                   created_at VARCHAR, mblogid VARCHAR, text_raw TEXT, \
                                   source VARCHAR, region_name VARCHAR, deleted BOOLEAN, \
                                   uid INTEGER, pic_ids VARCHAR, pic_num INTEGER, \
                                   retweeted_status INTEGER, url_struct json,topic_struct json, \
                                   tag_struct json, number_display_strategy json, \
                                   mix_media_info json, visible json, text TEXT, \
                                   attitudes_status INTEGER, showFeedRepost BOOLEAN, \
                                   showFeedComment BOOLEAN, pictureViewerSign BOOLEAN, \
                                   showPictureViewer BOOLEAN, favorited BOOLEAN, can_edit BOOLEAN, \
                                   is_paid BOOLEAN, share_repost_type INTEGER, rid VARCHAR, \
                                   pic_infos VARCHAR, cardid VARCHAR, pic_bg_new VARCHAR, \
                                   mark VARCHAR, mblog_vip_type INTEGER, reposts_count INTEGER, \
                                   comments_count INTEGER, attitudes_count INTEGER, \
                                   mlevel INTEGER, content_auth INTEGER, is_show_bulletin INTEGER, \
                                   repost_type INTEGER, edit_count INTEGER, mblogtype INTEGER, \
                                   textLength INTEGER, isLongText BOOLEAN, annotations json, \
                                   geo json, pic_focus_point json, page_info json, title json, \
                                   continue_tag json, comment_manage_info json, \
                                   client_only BOOLEAN NOT NULL DEFAULT false, \
                                   unfavorited BOOLEAN NOT NULL DEFAULT false); \
                                   CREATE TABLE IF NOT EXISTS users(id INTEGER PRIMARY KEY, \
                                   profile_url VARCHAR, screen_name VARCHAR, \
                                   profile_image_url VARCHAR, avatar_large VARCHAR, \
                                   avatar_hd VARCHAR, planet_video BOOLEAN, v_plus INTEGER, \
                                   pc_new INTEGER, verified BOOLEAN, verified_type INTEGER, \
                                   domain VARCHAR, weihao VARCHAR, verified_type_ext INTEGER, \
                                   follow_me BOOLEAN, following BOOLEAN, mbrank INTEGER, \
                                   mbtype INTEGER, icon_list VARCHAR); \
                                   CREATE TABLE IF NOT EXISTS picture_blob(\
                                   url VARCHAR PRIMARY KEY, id VARCHAR, blob BLOB);";
const DATABASE: &str = "res/weiback.db";

type DBResult<T> = std::result::Result<T, sqlx::Error>;

#[derive(Debug)]
pub struct Persister {
    db_path: PathBuf,
    db_pool: Option<SqlitePool>,
}

impl Persister {
    pub fn new() -> Self {
        Persister {
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
        } else {
            info!("db {:?} not exists, create it", self.db_path);
            Sqlite::create_database(self.db_path.to_str().unwrap()).await?;
            let mut db = SqliteConnection::connect(self.db_path.to_str().unwrap()).await?;
            use futures::stream::TryStreamExt;
            sqlx::query(DATABASE_CREATE_SQL)
                .execute_many(&mut db)
                .await
                .try_for_each_concurrent(None, |res| async move {
                    let query_result = res;
                    trace!(
                        "rows_affected {}, last_insert_rowid {}",
                        query_result.rows_affected(),
                        query_result.last_insert_rowid()
                    );
                    Ok(())
                })
                .await?;
        }
        self.db_pool = Some(
            SqlitePoolOptions::new()
                .min_connections(2)
                .connect_lazy(self.db_path.to_str().unwrap())?,
        );
        Ok(())
    }

    pub async fn insert_post(&self, post: &Post) -> Result<()> {
        trace!("insert post: {:?}", post);
        self._insert_post(post).await?;
        if post["user"]["id"].is_number() {
            self.insert_user(&post["user"]).await?;
        }
        if post["retweeted_status"].is_object() {
            self._insert_post(&post["retweeted_status"]).await?;
            if post["retweeted_status"]["user"]["id"].is_number() {
                self.insert_user(&post["retweeted_status"]["user"]).await?;
            }
        }
        Ok(())
    }

    pub async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        debug!("unfav post {} in db", id);
        sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
            .bind(id)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    pub async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        debug!("mark favorited post {} in db", id);
        sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
            .bind(id)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    pub async fn query_posts_to_unfavorite(&self, limit: u32, offset: u32) -> Result<Vec<i64>> {
        debug!("query posts to unfavorite, limit {limit} offset {offset}");
        Ok(sqlx::query_as::<Sqlite, (i64,)>(
            "SELECT id FROM posts WHERE unfavorited == false and favorited \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.db_pool.as_ref().unwrap())
        .await?
        .into_iter()
        .map(|t| t.0)
        .collect())
    }

    pub async fn insert_img(&self, url: &str, img: &[u8]) -> Result<()> {
        debug!("insert img: {url}");
        let id = pic_url_to_id(url);
        let url = strip_url_queries(url);
        let result = sqlx::query("INSERT OR IGNORE INTO picture_blob VALUES (?, ?, ?)")
            .bind(url)
            .bind(id)
            .bind(img)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        trace!("insert img {id}-{url}, result: {result:?}");
        Ok(())
    }

    pub async fn insert_user(&self, user: &User) -> Result<()> {
        trace!("user: {user:?}");
        let result = sqlx::query(
            "INSERT OR IGNORE INTO users VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user["id"].as_i64().unwrap())
        .bind(user["profile_url"].as_str().unwrap())
        .bind(user["screen_name"].as_str().unwrap())
        .bind(user["profile_image_url"].as_str().unwrap())
        .bind(user["avatar_large"].as_str())
        .bind(user["avatar_hd"].as_str().unwrap())
        .bind(user["planet_video"].as_bool())
        .bind(user["v_plus"].as_i64())
        .bind(user["pc_new"].as_i64())
        .bind(user["verified"].as_bool().unwrap())
        .bind(user["verified_type"].as_i64().unwrap())
        .bind(user["domain"].as_str())
        .bind(user["weihao"].as_str())
        .bind(user["verified_type_ext"].as_i64())
        .bind(user["follow_me"].as_bool().unwrap())
        .bind(user["following"].as_bool().unwrap())
        .bind(user["mbrank"].as_i64().unwrap())
        .bind(user["mbtype"].as_i64().unwrap())
        .bind(
            user["icon_list"]
                .is_object()
                .then_some(user["icon_list"].to_string()),
        )
        .execute(self.db_pool.as_ref().unwrap())
        .await?;
        trace!("insert user {user:?}, result {result:?}");
        Ok(())
    }

    pub async fn query_img(&self, url: &str) -> Result<Bytes> {
        debug!("query img: {url}");
        let result =
            sqlx::query_as::<Sqlite, PictureBlob>("SELECT * FROM picture_blob WHERE url = ?")
                .bind(url)
                .fetch_one(self.db_pool.as_ref().unwrap())
                .await;
        match result {
            Ok(res) => Ok(res.blob.into()),
            Err(sqlx::Error::RowNotFound) => Err(Error::NotInLocal),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn query_post(&self, id: i64) -> Result<Post> {
        debug!("query post, id: {id}");
        let sql_post = self._query_post(id).await;
        match sql_post {
            Ok(post) => Ok(self._sql_post_to_post(post).await?),
            Err(sqlx::Error::RowNotFound) => Err(Error::NotInLocal),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn query_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Posts> {
        debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
        let sql_posts = self._query_posts(limit, offset, reverse).await?;
        debug!("geted {} post from local", sql_posts.len());
        let data: Vec<_> = join_all(
            sql_posts
                .into_iter()
                .map(|p| async { self._sql_post_to_post(p).await }),
        )
        .await
        .into_iter()
        .collect::<std::result::Result<_, _>>()?;
        debug!("fetched {} posts", data.len());
        Ok(data)
    }

    #[allow(unused)]
    pub async fn query_user(&self, id: i64) -> Result<User> {
        match self._query_user(id).await {
            Ok(user) => Ok(user),
            Err(sqlx::Error::RowNotFound) => Err(Error::NotInLocal),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn query_db_total_num(&self) -> Result<u32> {
        Ok(
            sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
                .fetch_one(self.db_pool.as_ref().unwrap())
                .await?
                .0,
        )
    }
}

// ==================================================================================

// Private functions
impl Persister {
    async fn _insert_post(&self, post: &Value) -> DBResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO posts \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(post["id"].as_i64().unwrap())
        .bind(post["created_at"].as_str().unwrap())
        .bind(post["mblogid"].as_str())
        .bind(post["text_raw"].as_str().unwrap())
        .bind(post["source"].as_str().unwrap())
        .bind(post["region_name"].as_str())
        .bind(post["deleted"].as_bool().unwrap_or_default())
        .bind(post["user"]["id"].as_i64())
        .bind((post["pic_ids"].is_array()).then_some(post["pic_ids"].to_string()))
        .bind(post["pic_num"].as_i64())
        .bind(post["retweeted_status"]["id"].as_i64())
        .bind((post["url_struct"].is_object()).then_some(post["url_struct"].to_string()))
        .bind((post["topic_struct"].is_object()).then_some(post["topic_struct"].to_string()))
        .bind((post["tag_struct"].is_object()).then_some(post["tag_struct"].to_string()))
        .bind(
            (post["number_display_strategy"].is_object())
                .then_some(post["number_display_strategy"].to_string()),
        )
        .bind((post["mix_media_info"].is_object()).then_some(post["mix_media_info"].to_string()))
        .bind((post["visible"].is_object()).then_some(post["visible"].to_string()))
        .bind(post["text"].as_str())
        .bind(post["attitudes_status"].as_i64())
        .bind(post["showFeedRepost"].as_bool())
        .bind(post["showFeedComment"].as_bool())
        .bind(post["pictureViewerSign"].as_bool())
        .bind(post["showPictureViewer"].as_bool())
        .bind(post["favorited"].as_bool().unwrap_or_default())
        .bind(post["can_edit"].as_bool().unwrap_or_default())
        .bind(post["is_paid"].as_bool().unwrap_or_default())
        .bind(post["share_repost_type"].as_i64())
        .bind(post["rid"].as_str().unwrap_or_default())
        .bind((post["pic_infos"].is_object()).then_some(post["pic_infos"].to_string()))
        .bind(post["cardid"].as_str().unwrap_or_default())
        .bind(post["pic_bg_new"].as_str().unwrap_or_default())
        .bind(post["mark"].as_str().unwrap_or_default())
        .bind(post["mblog_vip_type"].as_i64())
        .bind(post["reposts_count"].as_i64())
        .bind(post["comments_count"].as_i64())
        .bind(post["attitudes_count"].as_i64())
        .bind(post["mlevel"].as_i64())
        .bind(post["content_auth"].as_i64())
        .bind(post["is_show_bulletin"].as_i64())
        .bind(post["repost_type"].as_i64())
        .bind(post["edit_count"].as_i64())
        .bind(post["mblogtype"].as_i64())
        .bind(post["textLength"].as_i64())
        .bind(post["isLongText"].as_bool().unwrap_or_default())
        .bind((post["annotations"].is_object()).then_some(post["annotations"].to_string()))
        .bind((post["geo"].is_object()).then_some(post["geo"].to_string()))
        .bind((post["pic_focus_point"].is_object()).then_some(post["pic_focus_point"].to_string()))
        .bind((post["page_info"].is_object()).then_some(post["page_info"].to_string()))
        .bind(post["title"].as_str())
        .bind((post["continue_tag"].is_object()).then_some(post["continue_tag"].to_string()))
        .bind(
            (post["comment_manage_info"].is_object())
                .then_some(post["comment_manage_info"].to_string()),
        )
        .bind(post["client_only"].as_bool().unwrap_or_default())
        .bind(post["unfavorited"].as_bool().unwrap_or_default())
        .execute(self.db_pool.as_ref().unwrap())
        .await?;
        Ok(())
    }

    async fn _query_user(&self, id: i64) -> DBResult<User> {
        let sql_user: SqlUser = sqlx::query_as(
            "SELECT id, profile_url, screen_name, profile_image_url, \
             avatar_large, avatar_hd FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_one(self.db_pool.as_ref().unwrap())
        .await?;

        let result = serde_json::to_value(sql_user).unwrap();
        // TODO: conv icon_list to Value
        Ok(result)
    }

    async fn _query_post(&self, id: i64) -> DBResult<SqlPost> {
        sqlx::query_as::<sqlx::Sqlite, SqlPost>(
            "SELECT id, created_at, mblogid, text_raw, source, region_name, \
            deleted, uid, pic_ids, pic_num, pic_infos, retweeted_status, url_struct, \
            topic_struct, tag_struct, number_display_strategy, mix_media_info, \
            isLongText, client_only FROM posts WHERE id = ?",
        )
        .bind(id)
        .fetch_one(self.db_pool.as_ref().unwrap())
        .await
    }

    async fn _query_posts(&self, limit: u32, offset: u32, reverse: bool) -> DBResult<Vec<SqlPost>> {
        let sql_expr = if reverse {
            "SELECT id, created_at, mblogid, text_raw, source, region_name, \
             deleted, uid, pic_ids, pic_num, pic_infos, retweeted_status, \
             url_struct, topic_struct, tag_struct, number_display_strategy, \
             mix_media_info, isLongText, client_only, unfavorited FROM posts \
             WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
        } else {
            "SELECT id, created_at, mblogid, text_raw, source, region_name, \
             deleted, uid, pic_ids, pic_num, pic_infos, retweeted_status, \
             url_struct, topic_struct, tag_struct, number_display_strategy, \
             mix_media_info, isLongText, client_only, unfavorited FROM posts \
             WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
        };
        sqlx::query_as::<sqlx::Sqlite, SqlPost>(sql_expr)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db_pool.as_ref().unwrap())
            .await
    }

    async fn _sql_post_to_post(&self, sql_post: SqlPost) -> DBResult<Post> {
        let user = if let Some(uid) = sql_post.uid {
            self._query_user(uid).await?
        } else {
            Value::Null
        };
        let retweet = if let Some(ret_id) = sql_post.retweeted_status {
            let sql_retweet = self._query_post(ret_id).await?;
            let user = if let Some(uid) = sql_retweet.uid {
                self._query_user(uid).await?
            } else {
                Value::Null
            };
            let mut ret = sql_post_to_post(sql_retweet);
            ret["user"] = user;
            ret
        } else {
            Value::Null
        };

        let mut post = sql_post_to_post(sql_post);
        post["user"] = user;
        if retweet.is_object() {
            post["retweeted_status"] = retweet;
        }

        Ok(post)
    }
}

fn sql_post_to_post(sql_post: SqlPost) -> Post {
    trace!("convert SqlPost to Post: {:?}", sql_post);
    let mut map = serde_json::Map::new();
    map.insert("id".into(), serde_json::to_value(sql_post.id).unwrap());
    map.insert("created_at".into(), to_value(sql_post.created_at).unwrap());
    map.insert("mblogid".into(), to_value(sql_post.mblogid).unwrap());
    map.insert("text_raw".into(), to_value(sql_post.text_raw).unwrap());
    map.insert("source".into(), to_value(sql_post.source).unwrap());
    if let Some(v) = sql_post.region_name {
        map.insert("region_name".into(), to_value(v).unwrap());
    }
    map.insert("deleted".into(), to_value(sql_post.deleted).unwrap());
    if let Some(v) = sql_post.pic_ids {
        map.insert("pic_ids".into(), from_str(&v).unwrap());
    }
    if let Some(v) = sql_post.pic_num {
        map.insert("pic_num".into(), to_value(v).unwrap());
    }
    if let Some(v) = sql_post.pic_infos {
        map.insert("pic_infos".into(), from_str(&v).unwrap());
    }
    if let Some(v) = sql_post.url_struct {
        map.insert("url_struct".into(), from_str(&v).unwrap());
    }
    if let Some(v) = sql_post.topic_struct {
        map.insert("topic_struct".into(), from_str(&v).unwrap());
    }
    if let Some(v) = sql_post.tag_struct {
        map.insert("tag_struct".into(), from_str(&v).unwrap());
    }
    if let Some(v) = sql_post.number_display_strategy {
        map.insert("number_display_strategy".into(), from_str(&v).unwrap());
    }
    if let Some(v) = sql_post.mix_media_info {
        map.insert("mix_media_info".into(), from_str(&v).unwrap());
    }
    map.insert(
        "isLongText".into(),
        to_value(sql_post.is_long_text).unwrap(),
    );
    map.insert(
        "client_only".into(),
        to_value(sql_post.client_only).unwrap(),
    );
    map.insert(
        "unfavorited".into(),
        to_value(sql_post.unfavorited).unwrap(),
    );

    Value::Object(map)
}

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct SqlPost {
    pub id: i64,
    pub created_at: String,
    pub mblogid: String,
    pub text_raw: String,
    pub source: String,
    pub region_name: Option<String>,
    pub deleted: bool,
    pub uid: Option<i64>,
    pub pic_ids: Option<String>,
    pub pic_num: Option<i64>,
    pub retweeted_status: Option<i64>,
    pub url_struct: Option<String>,
    pub topic_struct: Option<String>,
    pub tag_struct: Option<String>,
    pub number_display_strategy: Option<String>,
    pub mix_media_info: Option<String>,
    #[sqlx(default)]
    pub visible: String,
    #[sqlx(default)]
    pub text: String,
    #[sqlx(default)]
    pub attitudes_status: i64,
    #[sqlx(default, rename = "showFeedRepost")]
    pub show_feed_repost: bool,
    #[sqlx(default, rename = "showFeedComment")]
    pub show_feed_comment: bool,
    #[sqlx(default, rename = "pictureViewerSign")]
    pub picture_viewer_sign: bool,
    #[sqlx(default, rename = "showPictureViewer")]
    pub show_picture_viewer: bool,
    #[sqlx(default)]
    pub favorited: bool,
    #[sqlx(default)]
    pub can_edit: bool,
    #[sqlx(default)]
    pub is_paid: bool,
    #[sqlx(default)]
    pub share_repost_type: Option<i64>,
    #[sqlx(default)]
    pub rid: Option<String>,
    #[sqlx(default)]
    pub pic_infos: Option<String>,
    #[sqlx(default)]
    pub cardid: Option<String>,
    #[sqlx(default)]
    pub pic_bg_new: Option<String>,
    #[sqlx(default)]
    pub mark: Option<String>,
    #[sqlx(default)]
    pub mblog_vip_type: Option<i64>,
    #[sqlx(default)]
    pub reposts_count: Option<i64>,
    #[sqlx(default)]
    pub comments_count: Option<i64>,
    #[sqlx(default)]
    pub attitudes_count: Option<i64>,
    #[sqlx(default)]
    pub mlevel: Option<i64>,
    #[sqlx(default)]
    pub content_auth: Option<i64>,
    #[sqlx(default)]
    pub is_show_bulletin: Option<i64>,
    #[sqlx(default)]
    pub repost_type: Option<i64>,
    #[sqlx(default)]
    pub edit_count: Option<i64>,
    #[sqlx(default)]
    pub mblogtype: Option<i64>,
    #[sqlx(default, rename = "textLength")]
    pub text_length: Option<i64>,
    #[sqlx(default, rename = "isLongText")]
    pub is_long_text: bool,
    #[sqlx(default)]
    pub annotations: Option<String>,
    #[sqlx(default)]
    pub geo: Option<String>,
    #[sqlx(default)]
    pub pic_focus_point: Option<String>,
    #[sqlx(default)]
    pub page_info: Option<String>,
    #[sqlx(default)]
    pub title: Option<String>,
    #[sqlx(default)]
    pub continue_tag: Option<String>,
    #[sqlx(default)]
    pub comment_manage_info: Option<String>,
    #[sqlx(default)]
    pub client_only: bool,
    #[sqlx(default)]
    pub unfavorited: bool,
}

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct SqlUser {
    pub id: i64,
    pub profile_url: String,
    pub screen_name: String,
    pub profile_image_url: String,
    pub avatar_large: String,
    pub avatar_hd: String,
    #[sqlx(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    pub v_plus: i64,
    #[sqlx(default)]
    pub pc_new: i64,
    #[sqlx(default)]
    pub verified: bool,
    #[sqlx(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    pub domain: String,
    #[sqlx(default)]
    pub weihao: String,
    #[sqlx(default)]
    pub verified_type_ext: Option<i64>,
    #[sqlx(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    pub following: bool,
    #[sqlx(default)]
    pub mbrank: i64,
    #[sqlx(default)]
    pub mbtype: i64,
    #[sqlx(default)]
    pub icon_list: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct PictureBlob {
    pub url: String,
    pub id: String,
    pub blob: Vec<u8>,
}
