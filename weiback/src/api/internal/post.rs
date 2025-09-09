use std::borrow::Cow;
use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::{page_info::PageInfoInternal, url_struct::UrlStructInternal};
use crate::models::{Post, User, mix_media_info::MixMediaInfo, pic_infos::PicInfoItem};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PostInternal {
    pub attitudes_count: Option<i64>,
    #[serde(default)]
    pub attitudes_status: i64,
    #[serde(deserialize_with = "deserialize_created_at")]
    pub created_at: DateTime<FixedOffset>,
    pub comments_count: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_deleted")]
    pub deleted: bool,
    pub edit_count: Option<i64>,
    #[serde(default)]
    pub favorited: bool,
    pub geo: Option<Value>,
    pub id: i64,
    #[serde(default, rename = "isLongText")]
    pub is_long_text: bool,
    #[serde(default, rename = "longText")]
    pub long_text: Option<LongText>,
    pub mblogid: String,
    #[serde(default, deserialize_with = "deserialize_ids")]
    pub mix_media_ids: Option<Vec<String>>,
    pub mix_media_info: Option<MixMediaInfo>,
    pub page_info: Option<PageInfoInternal>,
    #[serde(default, deserialize_with = "deserialize_ids")]
    pub pic_ids: Option<Vec<String>>,
    pub pic_infos: Option<HashMap<String, PicInfoItem>>,
    pub pic_num: Option<i64>,
    pub region_name: Option<String>,
    pub reposts_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub retweeted_status: Option<Box<PostInternal>>,
    pub source: Option<String>,
    pub text: String,
    pub url_struct: Option<UrlStructInternal>,
    #[serde(default, deserialize_with = "deserialize_user")]
    pub user: Option<User>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LongText {
    pub content: String,
}

impl From<PostInternal> for Post {
    fn from(value: PostInternal) -> Self {
        Self {
            attitudes_count: value.attitudes_count,
            attitudes_status: value.attitudes_status,
            created_at: value.created_at,
            comments_count: value.comments_count,
            deleted: value.deleted,
            edit_count: value.edit_count,
            favorited: value.favorited,
            geo: value.geo,
            id: value.id,
            mblogid: value.mblogid,
            mix_media_ids: value.mix_media_ids,
            mix_media_info: value.mix_media_info,
            page_info: value.page_info.map(|p| p.into()),
            pic_ids: value.pic_ids,
            pic_infos: value.pic_infos,
            pic_num: value.pic_num,
            region_name: value.region_name,
            reposts_count: value.reposts_count,
            repost_type: value.repost_type,
            retweeted_status: value.retweeted_status.map(|r| Box::new((*r).into())),
            source: value.source,
            text: value.text,
            url_struct: value.url_struct.map(|u| u.into()),
            user: value.user,
        }
    }
}

fn deserialize_user<'de, D>(deserializer: D) -> std::result::Result<Option<User>, D::Error>
where
    D: Deserializer<'de>,
{
    let user = Option::<User>::deserialize(deserializer)?;
    Ok(user.and_then(|u| if u.id == 0 { None } else { Some(u) }))
}

fn deserialize_deleted<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s == "1")
}

pub fn deserialize_ids<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let ids = Option::<Vec<String>>::deserialize(deserializer)?;
    Ok(ids.and_then(|ids| if ids.is_empty() { None } else { Some(ids) }))
}

pub fn deserialize_created_at<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let created_at = Cow::<'_, str>::deserialize(deserializer)?;
    DateTime::parse_from_str(&created_at, "%a %b %d %T %z %Y").map_err(serde::de::Error::custom)
}
