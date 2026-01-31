use std::path::{Path, PathBuf};

use sqlx::{Sqlite, SqlitePool};
use url::Url;

use crate::error::{Error, Result};
use crate::models::PictureMeta;
use crate::storage::PictureInfo;
use crate::utils::{pic_url_to_id, url_to_db_key};

#[derive(sqlx::FromRow, Debug)]
struct PictureDbRecord {
    url: String,
    path: Option<String>,
    post_id: Option<i64>,
    user_id: Option<i64>,
}

impl TryFrom<PictureDbRecord> for PictureInfo {
    type Error = Error;

    fn try_from(record: PictureDbRecord) -> std::result::Result<Self, Self::Error> {
        let path = record.path.ok_or_else(|| {
            Error::DbError(format!("Picture path is NULL for URL {}", record.url))
        })?;
        let url_obj = Url::parse(&record.url)?;
        let meta = if let Some(user_id) = record.user_id {
            PictureMeta::Avatar {
                url: url_obj,
                user_id,
            }
        } else if let Some(post_id) = record.post_id {
            PictureMeta::InPost {
                url: url_obj,
                post_id,
            }
        } else {
            PictureMeta::Other { url: url_obj }
        };
        Ok(PictureInfo {
            meta,
            path: PathBuf::from(path),
        })
    }
}

pub async fn save_picture_meta(
    db: &SqlitePool,
    picture_meta: &PictureMeta,
    path: Option<&Path>,
) -> Result<()> {
    let (url, post_id, user_id) = match picture_meta {
        PictureMeta::Avatar { url, user_id } => (url, None, Some(user_id)),
        PictureMeta::InPost { url, post_id } => (url, Some(post_id), None),
        PictureMeta::Other { url } => (url, None, None),
    };
    let url_str = url_to_db_key(url).to_string();
    sqlx::query(
        r#"INSERT OR IGNORE INTO picture (
    id,
    path,
    post_id,
    url,
    user_id
)
VALUES
    (?, ?, ?, ?, ?);"#,
    )
    .bind(pic_url_to_id(picture_meta.url()).unwrap_or_default())
    .bind(path.map(|p| p.to_str()))
    .bind(post_id)
    .bind(url_str)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_picture_path(db: &SqlitePool, url: &Url) -> Result<Option<PathBuf>> {
    let raw_res =
        sqlx::query_scalar::<Sqlite, String>(r#"SELECT path FROM picture WHERE url = ?;"#)
            .bind(url.as_str())
            .fetch_optional(db)
            .await?;
    Ok(raw_res.map(PathBuf::from))
}

pub async fn get_pictures_by_post_id(db: &SqlitePool, post_id: i64) -> Result<Vec<PictureInfo>> {
    let records: Vec<PictureDbRecord> =
        sqlx::query_as("SELECT * FROM picture WHERE post_id = ? AND path IS NOT NULL")
            .bind(post_id)
            .fetch_all(db)
            .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_avatar_by_user_id(db: &SqlitePool, user_id: i64) -> Result<Option<PictureInfo>> {
    let record: Option<PictureDbRecord> = sqlx::query_as(
        "SELECT * FROM picture WHERE user_id = ? AND post_id IS NULL AND path IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    record.map(PictureInfo::try_from).transpose()
}

pub async fn get_pictures_by_ids(db: &SqlitePool, ids: &[String]) -> Result<Vec<PictureInfo>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT * FROM picture WHERE id IN ({}) AND path IS NOT NULL",
        placeholders
    );

    let mut query = sqlx::query_as::<_, PictureDbRecord>(&sql);
    for id in ids {
        query = query.bind(id);
    }
    let records = query.fetch_all(db).await?;

    records.into_iter().map(PictureInfo::try_from).collect()
}
