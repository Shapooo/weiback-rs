use std::path::{Path, PathBuf};

use sqlx::{Executor, Sqlite};
use url::Url;

use crate::error::Result;

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

#[cfg(test)]
mod local_tests {
    use super::*;
    use crate::storage::database::create_db_pool_with_url;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        create_db_pool_with_url("sqlite::memory:").await.unwrap()
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
