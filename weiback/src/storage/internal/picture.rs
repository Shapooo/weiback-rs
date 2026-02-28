use std::path::PathBuf;

use sqlx::{Executor, Sqlite};
use url::Url;

use crate::error::{Error, Result};
use crate::models::{PictureDefinition, PictureMeta};
use crate::storage::PictureInfo;
use crate::utils::{pic_url_to_db_key, pic_url_to_id};

#[derive(sqlx::FromRow, Debug)]
struct PictureDbRecord {
    url: String,
    path: Option<String>,
    post_id: Option<i64>,
    user_id: Option<i64>,
    definition: Option<String>,
}

impl TryFrom<PictureDbRecord> for PictureInfo {
    type Error = Error;

    fn try_from(record: PictureDbRecord) -> std::result::Result<Self, Self::Error> {
        let path = record.path.ok_or_else(|| {
            Error::DbError(format!("Picture path is NULL for URL {}", record.url))
        })?;
        let url_obj = Url::parse(&record.url)?;
        let meta = match record {
            PictureDbRecord {
                post_id: Some(post_id),
                definition: Some(definition),
                user_id: None,
                ..
            } => PictureMeta::Attached {
                url: url_obj,
                post_id,
                definition: PictureDefinition::from(definition.as_str()),
            },
            PictureDbRecord {
                post_id: Some(post_id),
                definition: None,
                user_id: None,
                ..
            } => PictureMeta::Cover {
                url: url_obj,
                post_id,
            },
            PictureDbRecord {
                user_id: Some(user_id),
                post_id: None,
                definition: None,
                ..
            } => PictureMeta::Avatar {
                url: url_obj,
                user_id,
            },
            PictureDbRecord {
                post_id: None,
                user_id: None,
                definition: None,
                ..
            } => PictureMeta::Other { url: url_obj },
            _ => unreachable!(),
        };
        Ok(PictureInfo {
            meta,
            path: PathBuf::from(path),
        })
    }
}

pub async fn save_picture_meta<'e, E>(
    executor: E,
    picture_meta: &PictureMeta,
    relative_path_str: Option<&str>,
) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (url, post_id, user_id, definition) = match picture_meta {
        PictureMeta::Attached {
            url,
            definition,
            post_id,
        } => (url, Some(*post_id), None, Some(definition)),
        PictureMeta::Cover { url, post_id } => (url, Some(*post_id), None, None),
        PictureMeta::Avatar { url, user_id } => (url, None, Some(*user_id), None),
        PictureMeta::Other { url } => (url, None, None, None),
    };
    let url_str = pic_url_to_db_key(url).to_string();
    sqlx::query(
        r#"INSERT OR IGNORE INTO picture (
    id,
    path,
    post_id,
    url,
    user_id,
    definition
)
VALUES
    (?, ?, ?, ?, ?, ?);"#,
    )
    .bind(pic_url_to_id(picture_meta.url()).unwrap_or_default())
    .bind(relative_path_str)
    .bind(post_id)
    .bind(url_str)
    .bind(user_id)
    .bind(definition.map(<&str>::from))
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_picture_path<'e, E>(executor: E, url: &Url) -> Result<Option<PathBuf>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let raw_res =
        sqlx::query_scalar::<Sqlite, String>(r#"SELECT path FROM picture WHERE url = ?;"#)
            .bind(url.as_str())
            .fetch_optional(executor)
            .await?;
    Ok(raw_res.map(PathBuf::from))
}

pub async fn get_users_with_duplicate_avatars<'e, E>(executor: E) -> Result<Vec<i64>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let ids = sqlx::query_scalar::<Sqlite, i64>(
        "SELECT user_id FROM picture WHERE user_id IS NOT NULL GROUP BY user_id HAVING COUNT(user_id) > 1",
    )
    .fetch_all(executor)
    .await?;
    Ok(ids)
}

pub async fn get_pictures_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let records: Vec<PictureDbRecord> =
        sqlx::query_as("SELECT * FROM picture WHERE post_id = ? AND path IS NOT NULL")
            .bind(post_id)
            .fetch_all(executor)
            .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_avatars_by_user_id<'e, E>(executor: E, user_id: i64) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let records: Vec<PictureDbRecord> = sqlx::query_as(
        "SELECT * FROM picture WHERE user_id = ? AND post_id IS NULL AND path IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(executor)
    .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_avatar_by_user_id<'e, E>(executor: E, user_id: i64) -> Result<Option<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let record: Option<PictureDbRecord> = sqlx::query_as(
        "SELECT * FROM picture WHERE user_id = ? AND post_id IS NULL AND path IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await?;
    record.map(PictureInfo::try_from).transpose()
}

pub async fn get_pictures_by_ids<'e, E>(executor: E, ids: &[String]) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
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
    let records = query.fetch_all(executor).await?;

    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn get_pictures_by_id<'e, E>(executor: E, id: &str) -> Result<Vec<PictureInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let records = sqlx::query_as::<_, PictureDbRecord>(
        "SELECT * FROM picture WHERE id = ? AND path IS NOT NULL",
    )
    .bind(id)
    .fetch_all(executor)
    .await?;
    records.into_iter().map(PictureInfo::try_from).collect()
}

pub async fn delete_pictures_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM picture WHERE post_id = ?")
        .bind(post_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn get_duplicate_pic_ids<'e, E>(executor: E) -> Result<Vec<String>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let ids = sqlx::query_scalar::<Sqlite, String>(
        "SELECT id FROM picture WHERE id != '' GROUP BY id HAVING COUNT(id) > 1",
    )
    .fetch_all(executor)
    .await?;
    Ok(ids)
}

pub async fn delete_picture_by_url<'e, E>(executor: E, url: &Url) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM picture WHERE url = ?")
        .bind(url.as_str())
        .execute(executor)
        .await?;
    Ok(())
}
