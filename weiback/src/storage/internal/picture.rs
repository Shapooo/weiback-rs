use std::path::{Path, PathBuf};

use sqlx::{Sqlite, SqlitePool};
use url::Url;

use crate::error::Result;
use crate::models::PictureMeta;
use crate::utils::{pic_url_to_id, url_to_db_key};

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
    let url = url_to_db_key(url);
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
    .bind(url.as_str())
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
