use crate::{
    data::Post,
    error::{Error, Result},
};

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use futures::future::join_all;
use log::{debug, trace};
use serde::Serialize;
use serde_json::{from_str, to_value, Value};
use sqlx::FromRow;

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct SqlPost {
    pub id: i64,
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
    pub created_at: i64,
    pub created_at_tz: String,
}

pub fn sql_post_to_post(sql_post: SqlPost) -> Post {
    trace!("convert SqlPost to Post: {:?}", sql_post);
    let mut map = serde_json::Map::new();
    map.insert("id".into(), serde_json::to_value(sql_post.id).unwrap());
    map.insert(
        "created_at".into(),
        to_value(
            DateTime::<FixedOffset>::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_opt(sql_post.created_at, 0).unwrap(),
                sql_post.created_at_tz.parse().unwrap(),
            )
            .to_string(),
        )
        .unwrap(),
    );
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

pub fn parse_created_at(created_at: &str) -> Result<DateTime<FixedOffset>> {
    match DateTime::parse_from_str(created_at, "%a %b %d %T %z %Y") {
        Ok(dt) => Ok(dt),
        Err(e) => Err(Error::MalFormat(format!("{e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_datetime() {
        parse_created_at("Mon May 29 19:29:32 +0800 2023").unwrap();
        parse_created_at("Mon May 29 19:45:00 +0800 2023").unwrap();
        parse_created_at("Tue May 30 04:07:49 +0800 2023").unwrap();
    }
}
