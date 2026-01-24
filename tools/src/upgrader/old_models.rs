use anyhow::{Result, anyhow};
use chrono::{DateTime, FixedOffset, TimeZone};
use futures::Stream;
use serde_json::Value;
use sqlx::SqlitePool;
use url::Url;
use weiback::{internals::storage_internal::post::PostInternal, models::User};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OldUser {
    pub id: i64,
    pub screen_name: Option<String>,
    pub profile_image_url: Option<String>,
    pub avatar_large: Option<String>,
    pub avatar_hd: Option<String>,
    pub domain: Option<String>,
    pub following: Option<bool>,
    pub follow_me: Option<bool>,
}

impl TryFrom<OldUser> for User {
    type Error = anyhow::Error;

    fn try_from(old: OldUser) -> Result<Self, Self::Error> {
        Ok(User {
            id: old.id,
            screen_name: old.screen_name.unwrap_or_default(),
            avatar_hd: Url::parse(
                old.avatar_hd
                    .ok_or_else(|| anyhow!("user {} missing avatar_hd", old.id))?
                    .as_str(),
            )?,
            avatar_large: Url::parse(
                old.avatar_large
                    .ok_or_else(|| anyhow!("user {} missing avatar_large", old.id))?
                    .as_str(),
            )?,
            profile_image_url: Url::parse(
                old.profile_image_url
                    .ok_or_else(|| anyhow!("user {} missing profile_image_url", old.id))?
                    .as_str(),
            )?,
            domain: old.domain.unwrap_or_default(),
            following: old.following.unwrap_or(false),
            follow_me: old.follow_me.unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OldPost {
    pub attitudes_count: Option<i64>,
    pub attitudes_status: i64,
    pub comments_count: Option<i64>,
    pub created_at: Option<String>,
    pub created_at_timestamp: Option<i64>,
    pub created_at_tz: Option<String>,
    pub deleted: bool,
    pub edit_count: Option<i64>,
    pub favorited: bool,
    pub geo: Option<Value>,
    pub id: i64,
    pub mblogid: String,
    pub mix_media_info: Option<Value>,
    pub page_info: Option<Value>,
    pub pic_ids: Option<Value>,
    pub pic_infos: Option<Value>,
    pub pic_num: Option<i64>,
    pub region_name: Option<String>,
    pub reposts_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub retweeted_id: Option<i64>,
    pub source: Option<String>,
    pub text_raw: String,
    pub uid: Option<i64>,
    pub url_struct: Option<Value>,
}

pub fn get_old_users(db: &SqlitePool) -> impl Stream<Item = Result<OldUser, sqlx::Error>> {
    sqlx::query_as::<_, OldUser>(
        r#"SELECT
    id,
    screen_name,
    profile_image_url,
    avatar_large,
    avatar_hd,
    domain,
    following,
    follow_me
FROM
    users;"#,
    )
    .fetch(db)
}

pub fn get_old_posts(
    db: &SqlitePool,
    old_version: i64,
) -> impl Stream<Item = Result<OldPost, sqlx::Error>> {
    let query = match old_version {
        0 => {
            r#"SELECT
    *,
    retweeted_status as retweeted_id,
    NULL as created_at_timestamp,
    NULL as created_at_tz
FROM
    posts;"#
        }
        1 => {
            r#"SELECT
    *,
    retweeted_status as retweeted_id,
    created_at as created_at_timestamp,
    NULL as created_at
FROM
    posts;"#
        }
        2 => {
            r#"SELECT
    *
FROM
    posts;"#
        }
        _ => unreachable!(),
    };
    sqlx::query_as::<_, OldPost>(query).fetch(db)
}

impl TryFrom<OldPost> for PostInternal {
    type Error = anyhow::Error;

    fn try_from(old: OldPost) -> Result<Self> {
        let created_at = if let Some(s) = old.created_at {
            // The old format is like "Sat Mar 28 22:38:44 +0800 2020"
            DateTime::parse_from_str(&s, "%a %b %d %T %z %Y")
                .or_else(|_| DateTime::parse_from_rfc3339(&s))?
                .to_rfc3339()
        } else if let Some(ts) = old.created_at_timestamp {
            let tz_str = old.created_at_tz.unwrap_or_else(|| "+08:00".to_string());
            let tz = tz_str.parse::<FixedOffset>()?;
            tz.timestamp_opt(ts, 0)
                .single()
                .ok_or_else(|| anyhow!("Invalid timestamp"))?
                .to_rfc3339()
        } else {
            return Err(anyhow!("Post {} has no creation date", old.id));
        };

        Ok(PostInternal {
            id: old.id,
            mblogid: old.mblogid,
            uid: old.uid,
            created_at,
            text: old.text_raw,
            reposts_count: old.reposts_count,
            comments_count: old.comments_count,
            attitudes_count: old.attitudes_count,
            attitudes_status: old.attitudes_status,
            deleted: old.deleted,
            favorited: old.favorited,
            edit_count: old.edit_count,
            geo: old.geo,
            mix_media_info: old.mix_media_info,
            page_info: old.page_info,
            pic_ids: old.pic_ids,
            pic_infos: old.pic_infos,
            pic_num: old.pic_num,
            region_name: old.region_name,
            repost_type: old.repost_type,
            retweeted_id: old.retweeted_id,
            source: old.source,
            url_struct: old.url_struct,
            mix_media_ids: None, // This was not in old DB
        })
    }
}
