use std::path::{Path, PathBuf};

use log::info;
use sqlx::{Sqlite, SqlitePool};
use url::Url;

use crate::error::Result;
use crate::models::PictureMeta;
use crate::utils::pic_url_to_id;

pub async fn create_picture_table(db: &SqlitePool) -> Result<()> {
    info!("Creating post table if not exists...");
    sqlx::query(
        r#"CREATE TABLE
    IF NOT EXISTS picture (
        id TEXT,
        path TEXT,
        post_id TEXT,
        url TEXT PRIMARY KEY,
        user_id TEXT
    );"#,
    )
    .execute(db)
    .await?;
    info!("Picture table created successfully.");
    Ok(())
}

pub async fn save_picture_meta(
    db: &SqlitePool,
    picture_meta: &PictureMeta,
    path: Option<&Path>,
) -> Result<()> {
    let (post_id, user_id) = match picture_meta {
        PictureMeta::Avatar { url: _, user_id } => (None, Some(user_id)),
        PictureMeta::InPost { url: _, post_id } => (Some(post_id), None),
        PictureMeta::Other { url: _ } => (None, None),
    };
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
    .bind(picture_meta.url().as_str())
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_picture_path(db: &SqlitePool, url: &Url) -> Result<Option<PathBuf>> {
    let raw_res = sqlx::query_as::<Sqlite, (String,)>(r#"SELECT path FROM picture WHERE url = ?;"#)
        .bind(url.as_str())
        .fetch_optional(db)
        .await?;
    Ok(raw_res.map(|s| PathBuf::from(s.0)))
}
