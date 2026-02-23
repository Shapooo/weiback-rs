use std::path::{Path, PathBuf};

use sqlx::{Executor, Sqlite};
use url::Url;

use crate::{error::Result, utils::pic_url_to_db_key};

pub async fn get_video_paths_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<Vec<PathBuf>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let paths: Vec<String> = sqlx::query_scalar("SELECT path FROM video WHERE post_id = ?")
        .bind(post_id)
        .fetch_all(executor)
        .await?;
    Ok(paths.into_iter().map(PathBuf::from).collect())
}

pub async fn delete_videos_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM video WHERE post_id = ?")
        .bind(post_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn save_video_meta<'e, E>(executor: E, url: &Url, post_id: i64, path: &Path) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
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
    .bind(post_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_video_path<'e, E>(executor: E, url: &Url) -> Result<Option<PathBuf>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let url = pic_url_to_db_key(url);
    let raw_res: Option<String> =
        sqlx::query_scalar::<Sqlite, String>(r#"SELECT path FROM video WHERE url = ?;"#)
            .bind(url.as_str())
            .fetch_optional(executor)
            .await?;
    Ok(raw_res.map(PathBuf::from))
}

pub async fn delete_video_by_url<'e, E>(executor: E, url: &Url) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM video WHERE url = ?")
        .bind(url.as_str())
        .execute(executor)
        .await?;
    Ok(())
}
