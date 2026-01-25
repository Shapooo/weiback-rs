use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, FixedOffset, TimeZone};
use serde::Deserialize;
use serde_json::Value;
use sqlx::SqlitePool;
use url::Url;

use weiback::{
    internals::{
        page_info::PageInfoInternal, storage_internal::post::PostInternal,
        url_struct::UrlStructInternal,
    },
    models::{
        common::PicInfoDetail,
        mix_media_info::MixMediaInfo,
        page_info::PageInfo,
        pic_infos::{PicInfoItem, PicInfoType},
        url_struct::UrlStruct,
    },
};

#[derive(Debug, Deserialize)]
struct OldPicInfoItem {
    pic_id: String,
    r#type: PicInfoType,
    bmiddle: OldPicInfoDetail,
    large: OldPicInfoDetail,
}

#[derive(Debug, Deserialize)]
struct OldPicInfoDetail {
    url: String,
}

fn create_pic_info_detail(id: &str, host: &str, clarity_type: &str, ext: &str) -> PicInfoDetail {
    let url_str = format!("https://{}/{}/{}.{}", host, clarity_type, id, ext);
    PicInfoDetail {
        height: 0, // Default value
        width: 0,  // Default value
        url: url_str.parse().unwrap_or_else(|_| {
            log::warn!("Failed to parse reconstructed URL: {}", url_str);
            // Fallback to a dummy URL or handle error
            "https://example.com/placeholder.jpg".parse().unwrap()
        }),
    }
}

fn extract_host_and_ext(url_str: &str) -> Result<(String, String)> {
    let url = url_str.parse::<Url>()?;
    let host = url.host_str().context("URL has no host")?.to_string();
    let Some(ext) = url
        .path_segments()
        .and_then(|mut s| s.next_back())
        .and_then(|filename| filename.rsplit('.').next())
    else {
        return Err(anyhow!("URL has no file extension"));
    };
    Ok((host, ext.to_string()))
}

impl TryFrom<OldPicInfoItem> for PicInfoItem {
    type Error = anyhow::Error;

    fn try_from(old: OldPicInfoItem) -> Result<Self> {
        let id = old.pic_id;

        // Try to extract host and ext from a valid URL
        let (host, ext) = if let Ok((h, e)) = extract_host_and_ext(&old.large.url) {
            (h, e)
        } else if let Ok((h, e)) = extract_host_and_ext(&old.bmiddle.url) {
            (h, e)
        } else {
            return Err(anyhow!(
                "Could not extract host and extension from pic_id {}",
                id
            ));
        };

        Ok(Self {
            pic_id: id.clone(),
            object_id: id.clone(),
            r#type: old.r#type,
            bmiddle: create_pic_info_detail(&id, &host, "bmiddle", &ext),
            large: create_pic_info_detail(&id, &host, "large", &ext),
            largest: create_pic_info_detail(&id, &host, "largest", &ext),
            original: create_pic_info_detail(&id, &host, "original", &ext),
            thumbnail: create_pic_info_detail(&id, &host, "thumbnail", &ext),
            mw2000: create_pic_info_detail(&id, &host, "mw2000", &ext),
            focus_point: Default::default(),
            photo_tag: Default::default(),
            pic_status: Default::default(),
            video: Default::default(),
            video_hd: Default::default(),
            video_object_id: Default::default(),
            fid: Default::default(),
        })
    }
}

fn deserialize_pic_infos(
    value: Value,
    post_id: i64,
) -> Result<Option<HashMap<String, PicInfoItem>>> {
    let modern_format: Result<HashMap<String, PicInfoItem>, _> =
        serde_json::from_value(value.clone());
    if let Ok(pic_infos) = modern_format {
        return Ok(Some(pic_infos));
    }

    let legacy_format: Result<HashMap<String, OldPicInfoItem>, _> = serde_json::from_value(value);
    match legacy_format {
        Ok(legacy_pic_infos) => {
            let pic_infos = legacy_pic_infos
                .into_iter()
                .filter_map(|(id, old_item)| match old_item.try_into() {
                    Ok(item) => Some((id, item)),
                    Err(e) => {
                        log::warn!(
                            "post {post_id}, failed to convert OldPicInfoItem for id {id}: {e}"
                        );
                        None
                    }
                })
                .collect();
            Ok(Some(pic_infos))
        }
        Err(e) => {
            log::warn!(
                "post {post_id}, can't parse pic_infos as modern or legacy format, err: {e}"
            );
            Ok(None)
        }
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

pub async fn get_old_posts_paged(
    db: &SqlitePool,
    old_version: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<OldPost>> {
    let query = match old_version {
        0 => {
            r#"SELECT
    *,
    retweeted_status as retweeted_id,
    NULL as created_at_timestamp,
    NULL as created_at_tz
FROM
    posts
ORDER BY
    id
LIMIT
    ?
OFFSET
    ?;"#
        }
        1 => {
            r#"SELECT
    *,
    retweeted_status as retweeted_id,
    created_at as created_at_timestamp,
    NULL as created_at
FROM
    posts
ORDER BY
    id
LIMIT
    ?
OFFSET
    ?;"#
        }
        2 => {
            r#"SELECT
    *
FROM
    posts
ORDER BY
    id
LIMIT
    ?
OFFSET
    ?;"#
        }
        _ => unreachable!(),
    };
    Ok(sqlx::query_as(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?)
}

impl TryFrom<OldPost> for PostInternal {
    type Error = anyhow::Error;

    fn try_from(old: OldPost) -> Result<Self> {
        let id = old.id;
        let created_at = if let Some(s) = old.created_at {
            // The old format like "Sat Mar 28 22:38:44 +0800 2020"
            DateTime::parse_from_str(&s, "%a %b %d %T %z %Y")
                // The old format like "2020-07-08 22:38:44 +0800"
                .or_else(|_| DateTime::parse_from_str(&s, "%Y-%m-%d %T %:z"))
                .or_else(|_| DateTime::parse_from_rfc3339(&s))
                .with_context(|| format!("post {id}, parsing created_at {s}"))?
                .to_rfc3339()
        } else if let Some(ts) = old.created_at_timestamp {
            let tz_str = old.created_at_tz.unwrap_or_else(|| "+08:00".to_string());
            let tz = tz_str
                .parse::<FixedOffset>()
                .with_context(|| format!("post {id}, parsing created_at_tz"))?;
            tz.timestamp_opt(ts, 0)
                .single()
                .ok_or_else(|| anyhow!("Invalid timestamp"))
                .with_context(|| format!("post {id}, converting timestamp"))?
                .to_rfc3339()
        } else {
            return Err(anyhow!("Post {} has no creation date", id));
        };

        let page_info = old
            .page_info
            .map(|v| {
                let internal: PageInfoInternal = serde_json::from_value(v)
                    .with_context(|| format!("post {id}, deserializing PageInfoInternal"))?;
                let model: PageInfo = internal.into();
                serde_json::to_value(model)
                    .with_context(|| format!("post {id}, serializing PageInfo"))
            })
            .transpose()?;

        let url_struct = old
            .url_struct
            .map(|v| {
                let internal: UrlStructInternal = serde_json::from_value(v)
                    .with_context(|| format!("post {id}, deserializing UrlStructInternal"))?;
                let model: UrlStruct = internal
                    .try_into()
                    .with_context(|| format!("post {id}, converting UrlStruct"))?;
                serde_json::to_value(model)
                    .with_context(|| format!("post {id}, serializing UrlStruct"))
            })
            .transpose()?;

        let pic_infos = old
            .pic_infos
            .map(|v| {
                deserialize_pic_infos(v, id).and_then(|op| {
                    op.map(|v| {
                        serde_json::to_value(v)
                            .with_context(|| format!("post {id}, serializing PicInfoItem"))
                    })
                    .transpose()
                })
            })
            .transpose()?
            .flatten();

        let mix_media_info_model = old
            .mix_media_info
            .map(|v| {
                let model: MixMediaInfo = serde_json::from_value(v)
                    .with_context(|| format!("post {id}, deserializing MixMediaInfo"))?;
                anyhow::Ok(model)
            })
            .transpose()?;

        // Extract mix_media_ids from the model, if available
        let mix_media_ids = mix_media_info_model.as_ref().map(|mmi| {
            mmi.items
                .iter()
                .map(|item| match item {
                    weiback::models::mix_media_info::MixMediaInfoItem::Pic { id, .. } => id.clone(),
                    weiback::models::mix_media_info::MixMediaInfoItem::Video { id, .. } => {
                        id.clone()
                    }
                })
                .collect()
        });

        // Re-serialize the MixMediaInfo model back to a Value
        let mix_media_info = mix_media_info_model
            .map(|model| {
                serde_json::to_value(model)
                    .with_context(|| format!("post {id}, serializing MixMediaInfo"))
            })
            .transpose()?;

        Ok(PostInternal {
            id,
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
            mix_media_info,
            page_info,
            pic_ids: old.pic_ids,
            pic_infos,
            pic_num: old.pic_num,
            region_name: old.region_name,
            repost_type: old.repost_type,
            retweeted_id: old.retweeted_id,
            source: old.source,
            url_struct,
            mix_media_ids,
        })
    }
}
