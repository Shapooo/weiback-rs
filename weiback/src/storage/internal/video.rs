//! This module provides functions for interacting with the `video` table in the database.
//!
//! It handles the storage, retrieval, and deletion of video metadata, including their associated posts.
//!
//! # Table Structure: `video`
//!
//! | Column     | Type    | Description                                       |
//! |------------|---------|---------------------------------------------------|
//! | `url`      | `TEXT`  | The original URL of the video. **Primary Key.**   |
//! | `path`     | `TEXT`  | The local path where the video is stored.         |
//! | `post_id`  | `INTEGER` | The ID of the post this video is attached to.     |
//!
//! The `url` column serves as the primary key for uniqueness.

use std::path::{Path, PathBuf};

use sea_query::{Expr, OnConflict, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Executor, Sqlite};
use url::Url;

use crate::error::Result;

#[derive(sea_query::Iden)]
#[iden = "video"]
enum VideoIden {
    Table,
    Url,
    Path,
    PostId,
}

/// Retrieves the local paths of all videos associated with a specific post ID.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `post_id` - The ID of the post to retrieve video paths for.
///
/// # Returns
///
/// A `Result` containing a `Vec<PathBuf>` of video paths.
pub async fn get_video_paths_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<Vec<PathBuf>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::select()
        .column(VideoIden::Path)
        .from(VideoIden::Table)
        .and_where(Expr::col(VideoIden::PostId).eq(post_id))
        .build_sqlx(SqliteQueryBuilder);
    let paths: Vec<String> = sqlx::query_scalar_with(&sql, values)
        .fetch_all(executor)
        .await?;
    Ok(paths.into_iter().map(PathBuf::from).collect())
}

/// Deletes all video entries associated with a given post ID from the database.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `post_id` - The ID of the post whose videos are to be deleted.
///
/// # Returns
///
/// A `Result` indicating success or failure.
pub async fn delete_videos_by_post_id<'e, E>(executor: E, post_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::delete()
        .from_table(VideoIden::Table)
        .and_where(Expr::col(VideoIden::PostId).eq(post_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(executor).await?;
    Ok(())
}

/// Saves video metadata into the database.
///
/// If a video with the same URL already exists, it will be ignored.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `url` - The URL of the video.
/// * `post_id` - The ID of the post this video belongs to.
/// * `path` - The local path where the video is stored.
///
/// # Returns
///
/// A `Result` indicating success or failure.
pub async fn save_video_meta<'e, E>(executor: E, url: &Url, post_id: i64, path: &Path) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::insert()
        .into_table(VideoIden::Table)
        .columns([VideoIden::Url, VideoIden::Path, VideoIden::PostId])
        .values_panic([url.as_str().into(), path.to_str().into(), post_id.into()])
        .on_conflict(OnConflict::column(VideoIden::Url).do_nothing().to_owned())
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(executor).await?;
    Ok(())
}

/// Retrieves the local path of a video given its URL.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `url` - The URL of the video to retrieve the path for.
///
/// # Returns
///
/// A `Result` containing an `Option<PathBuf>`. `Some(PathBuf)` if the video is found, `None` otherwise.
pub async fn get_video_path<'e, E>(executor: E, url: &Url) -> Result<Option<PathBuf>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::select()
        .column(VideoIden::Path)
        .from(VideoIden::Table)
        .and_where(Expr::col(VideoIden::Url).eq(url.as_str()))
        .build_sqlx(SqliteQueryBuilder);
    let raw_res: Option<String> = sqlx::query_scalar_with(&sql, values)
        .fetch_optional(executor)
        .await?;
    Ok(raw_res.map(PathBuf::from))
}

/// Deletes a specific video entry from the database using its URL.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `url` - The URL of the video to be deleted.
///
/// # Returns
///
/// A `Result` indicating success or failure.
pub async fn delete_video_by_url<'e, E>(executor: E, url: &Url) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::delete()
        .from_table(VideoIden::Table)
        .and_where(Expr::col(VideoIden::Url).eq(url.as_str()))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(executor).await?;
    Ok(())
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use crate::storage::database::create_db_pool_with_url;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        create_db_pool_with_url(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_save_and_get_video() {
        let db = setup_db().await;
        let url = Url::parse("http://example.com/video.mp4").unwrap();
        let post_id = 123;
        let path = Path::new("videos/video.mp4");

        save_video_meta(&db, &url, post_id, path).await.unwrap();

        let retrieved_path = get_video_path(&db, &url).await.unwrap();
        assert_eq!(retrieved_path, Some(path.to_path_buf()));
    }

    #[tokio::test]
    async fn test_get_video_paths_by_post_id() {
        let db = setup_db().await;
        let post_id = 456;
        let url1 = Url::parse("http://example.com/video1.mp4").unwrap();
        let path1 = Path::new("videos/video1.mp4");
        let url2 = Url::parse("http://example.com/video2.mp4").unwrap();
        let path2 = Path::new("videos/video2.mp4");

        save_video_meta(&db, &url1, post_id, path1).await.unwrap();
        save_video_meta(&db, &url2, post_id, path2).await.unwrap();

        let paths = get_video_paths_by_post_id(&db, post_id).await.unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&path1.to_path_buf()));
        assert!(paths.contains(&path2.to_path_buf()));
    }

    #[tokio::test]
    async fn test_delete_videos_by_post_id() {
        let db = setup_db().await;
        let post_id = 789;
        let url = Url::parse("http://example.com/video_to_delete.mp4").unwrap();
        let path = Path::new("videos/video_to_delete.mp4");

        save_video_meta(&db, &url, post_id, path).await.unwrap();
        let paths_before = get_video_paths_by_post_id(&db, post_id).await.unwrap();
        assert_eq!(paths_before.len(), 1);

        delete_videos_by_post_id(&db, post_id).await.unwrap();

        let paths_after = get_video_paths_by_post_id(&db, post_id).await.unwrap();
        assert!(paths_after.is_empty());
    }

    #[tokio::test]
    async fn test_delete_video_by_url() {
        let db = setup_db().await;
        let post_id = 101;
        let url = Url::parse("http://example.com/another_video.mp4").unwrap();
        let path = Path::new("videos/another_video.mp4");

        save_video_meta(&db, &url, post_id, path).await.unwrap();
        let path_before = get_video_path(&db, &url).await.unwrap();
        assert!(path_before.is_some());

        delete_video_by_url(&db, &url).await.unwrap();

        let path_after = get_video_path(&db, &url).await.unwrap();
        assert!(path_after.is_none());
    }
}
