use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PostStorage {
    pub id: i64,
    pub mblogid: String,
    pub source: String,
    pub region_name: Option<String>,
    pub deleted: bool,
    pub pic_ids: Option<Value>,
    pub pic_num: Option<i64>,
    pub url_struct: Option<Value>,
    pub topic_struct: Option<Value>,
    pub tag_struct: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub text: String,
    #[sqlx(default)]
    pub attitudes_status: i64,
    #[sqlx(default)]
    pub favorited: bool,
    pub pic_infos: Option<Value>,
    pub reposts_count: Option<i64>,
    pub comments_count: Option<i64>,
    pub attitudes_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub edit_count: Option<i64>,
    #[sqlx(default, rename = "isLongText")]
    pub is_long_text: bool,
    pub geo: Option<Value>,
    pub page_info: Option<Value>,
    #[sqlx(default)]
    pub unfavorited: bool,
    pub created_at: String,
    pub retweeted_id: Option<i64>,
    #[sqlx(skip)]
    pub uid: Option<i64>,
}
