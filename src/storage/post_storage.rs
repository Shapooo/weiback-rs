use std::ops::DerefMut;

use serde_json::{Value, from_str, to_string};
use sqlx::{Executor, FromRow, Row, Sqlite};

use crate::error::{Error, Result};
use crate::models::Post;

#[derive(Debug, Clone, PartialEq)]
pub struct PostStorage {
    pub id: i64,
    pub mblogid: String,
    pub source: Option<String>,
    pub region_name: Option<String>,
    pub deleted: bool,
    pub pic_ids: Option<String>,
    pub pic_num: Option<i64>,
    pub url_struct: Option<Value>,
    pub topic_struct: Option<Value>,
    pub tag_struct: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub text: String,
    pub attitudes_status: i64,
    pub favorited: bool,
    pub pic_infos: Option<String>,
    pub reposts_count: Option<i64>,
    pub comments_count: Option<i64>,
    pub attitudes_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub edit_count: Option<i64>,
    pub is_long_text: bool,
    pub geo: Option<Value>,
    pub page_info: Option<Value>,
    pub unfavorited: bool,
    pub created_at: String,
    pub retweeted_id: Option<i64>,
    pub uid: Option<i64>,
}

impl TryFrom<Post> for PostStorage {
    type Error = Error;
    fn try_from(post: Post) -> Result<Self> {
        Ok(Self {
            id: post.id,
            mblogid: post.mblogid,
            source: post.source,
            region_name: post.region_name,
            deleted: post.deleted,
            pic_ids: post.pic_ids.map(|v| to_string(&v)).transpose()?,
            pic_num: post.pic_num,
            url_struct: post.url_struct,
            topic_struct: post.topic_struct,
            tag_struct: post.tag_struct,
            number_display_strategy: post.number_display_strategy,
            mix_media_info: post.mix_media_info,
            text: post.text,
            attitudes_status: post.attitudes_status,
            favorited: post.favorited,
            pic_infos: post.pic_infos.map(|h| to_string(&h)).transpose()?,
            reposts_count: post.reposts_count,
            comments_count: post.comments_count,
            attitudes_count: post.attitudes_count,
            repost_type: post.repost_type,
            edit_count: post.edit_count,
            is_long_text: post.is_long_text,
            geo: post.geo,
            page_info: post.page_info,
            unfavorited: post.unfavorited,
            created_at: post.created_at.to_string(), //TODO
            retweeted_id: post.retweeted_status.map(|r| r.id),
            uid: post.attitudes_count,
        })
    }
}

impl TryInto<Post> for PostStorage {
    type Error = Error;
    fn try_into(self) -> Result<Post> {
        Ok(Post {
            id: self.id,
            mblogid: self.mblogid,
            source: self.source,
            region_name: self.region_name,
            deleted: self.deleted,
            pic_ids: self.pic_ids.map(|s| from_str(&s)).transpose()?,
            pic_num: self.pic_num,
            url_struct: self.url_struct,
            topic_struct: self.topic_struct,
            tag_struct: self.tag_struct,
            number_display_strategy: self.number_display_strategy,
            mix_media_info: self.mix_media_info,
            text: self.text,
            attitudes_status: self.attitudes_status,
            favorited: self.favorited,
            pic_infos: self.pic_infos.map(|s| from_str(&s)).transpose()?,
            reposts_count: self.reposts_count,
            comments_count: self.comments_count,
            attitudes_count: self.attitudes_count,
            repost_type: self.repost_type,
            edit_count: self.edit_count,
            is_long_text: self.is_long_text,
            geo: self.geo,
            page_info: self.page_info,
            unfavorited: self.unfavorited,
            created_at: Default::default(), //TODO
            retweeted_status: None,
            user: None,
        })
    }
}

impl<'r, R> FromRow<'r, R> for PostStorage
where
    R: Row,
    for<'c> &'c str: sqlx::ColumnIndex<R>,
    i64: for<'c> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    String: for<'c> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    Option<String>: for<'c> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    bool: for<'c> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    Option<i64>: for<'c> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    Option<Value>: for<'c> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    fn from_row(row: &'r R) -> std::result::Result<Self, sqlx::Error> {
        Ok(PostStorage {
            id: row.try_get("id")?,
            mblogid: row.try_get("mblogid")?,
            source: row.try_get("source")?,
            region_name: row.try_get("region_name")?,
            deleted: row.try_get("deleted")?,
            pic_ids: row.try_get("pic_ids")?,
            pic_num: row.try_get("pic_num")?,
            url_struct: row.try_get("url_struct")?,
            topic_struct: row.try_get("topic_struct")?,
            tag_struct: row.try_get("tag_struct")?,
            number_display_strategy: row.try_get("number_display_strategy")?,
            mix_media_info: row.try_get("mix_media_info")?,
            text: row.try_get("text")?,
            attitudes_status: row.try_get("attitudes_status")?,
            favorited: row.try_get("favorited")?,
            pic_infos: row.try_get("pic_infos")?,
            reposts_count: row.try_get("reposts_count")?,
            comments_count: row.try_get("comments_count")?,
            attitudes_count: row.try_get("attitudes_count")?,
            repost_type: row.try_get("repost_type")?,
            edit_count: row.try_get("edit_count")?,
            is_long_text: row.try_get("isLongText")?,
            geo: row.try_get("geo")?,
            page_info: row.try_get("page_info")?,
            unfavorited: row.try_get("unfavorited")?,
            created_at: row.try_get("created_at")?,
            retweeted_id: row.try_get("retweeted_id")?,
            uid: row.try_get("uid")?,
        })
    }
}

pub async fn create_post_table<E>(mut executor: E) -> Result<()>
where
    E: DerefMut,
    for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
{
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts ( \
             id INTEGER PRIMARY KEY, \
             mblogid TEXT, \
             source TEXT, \
             region_name TEXT, \
             deleted INTEGER, \
             pic_ids TEXT, \
             pic_num INTEGER, \
             url_struct TEXT, \
             topic_struct TEXT, \
             tag_struct TEXT, \
             number_display_strategy TEXT, \
             mix_media_info TEXT, \
             text TEXT, \
             attitudes_status INTEGER, \
             favorited INTEGER, \
             pic_infos TEXT, \
             reposts_count INTEGER, \
             comments_count INTEGER, \
             attitudes_count INTEGER, \
             repost_type INTEGER, \
             edit_count INTEGER, \
             isLongText INTEGER, \
             geo TEXT, \
             page_info TEXT, \
             unfavorited INTEGER, \
             created_at TEXT, \
             retweeted_id INTEGER, \
             uid INTEGER \
             )",
    )
    .execute(&mut *executor)
    .await?;
    Ok(())
}
