use std::path::{Path, PathBuf};

use sqlx::{Sqlite, SqlitePool};
use url::Url;

use crate::{error::Result, utils::url_to_db_key};

pub async fn save_video_meta(db: &SqlitePool, url: &Url, post_id: i64, path: &Path) -> Result<()> {
    sqlx::query::<Sqlite>(
        r#"INSERT OR IGNORE INTO video (
    url,
    path,
    post_id
)
VALUES
    (?, ?, ?);"#,
    )
    .bind(url.as_str())
    .bind(path.to_str())
    .bind(post_id.to_string())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_video_path(db: &SqlitePool, url: &Url) -> Result<Option<PathBuf>> {
    let url = url_to_db_key(url);
    let raw_res: Option<String> =
        sqlx::query_scalar::<Sqlite, String>(r#"SELECT path FROM video WHERE url = ?;"#)
            .bind(url.as_str())
            .fetch_optional(db)
            .await?;
    Ok(raw_res.map(PathBuf::from))
}
