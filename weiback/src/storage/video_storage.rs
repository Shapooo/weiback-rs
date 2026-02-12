use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::{debug, warn};
use sqlx::{Acquire, Executor, Sqlite};
use url::Url;

use super::internal::video;
use crate::error::{Error, Result};
use crate::models::Video;
use crate::utils::url_to_path_str;

#[derive(Debug, Clone, Default)]
pub struct FileSystemVideoStorage;

impl FileSystemVideoStorage {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemVideoStorage {
    pub async fn get_video_blob<'c, A>(
        &self,
        video_path: &Path,
        acquirer: A,
        url: &Url,
    ) -> Result<Option<Bytes>>
    where
        A: Acquire<'c, Database = Sqlite>,
    {
        let mut conn = acquirer.acquire().await?;
        let Some(relative_path) = video::get_video_path(&mut *conn, url).await? else {
            return Ok(None);
        };
        let absolute_path = video_path.join(relative_path);
        match tokio::fs::read(&absolute_path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!(
                    "video file not found at {:?}, deleting db entry",
                    absolute_path
                );
                video::delete_video_by_url(&mut *conn, url).await?;
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save_video<'e, E>(
        &self,
        video_path: &Path,
        executor: E,
        video: &Video,
    ) -> Result<()>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let url = video.meta.url();
        let relative_path = PathBuf::from(url_to_path_str(url));
        let absolute_path = video_path.join(&relative_path);
        create_dir_all(
            absolute_path
                .parent()
                .ok_or(Error::Io(std::io::Error::other(
                    "cannot get parent of video path",
                )))?,
        )?;
        if let Some(parent) = absolute_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&absolute_path, &video.blob).await?;
        video::save_video_meta(executor, url, video.meta.post_id, relative_path.as_path()).await?;
        debug!("video {} saved to {:?}", video.meta.url(), absolute_path);
        Ok(())
    }

    pub async fn video_saved<'c, A>(
        &self,
        video_path: &Path,
        acquirer: A,
        url: &Url,
    ) -> Result<bool>
    where
        A: Acquire<'c, Database = Sqlite>,
    {
        let mut conn = acquirer.acquire().await?;
        let Some(relative_path) = video::get_video_path(&mut *conn, url).await? else {
            return Ok(false);
        };
        let absolute_path = video_path.join(relative_path);
        if absolute_path.exists() {
            Ok(true)
        } else {
            warn!(
                "video file not found at {:?}, deleting db entry",
                absolute_path
            );
            video::delete_video_by_url(&mut *conn, url).await?;
            Ok(false)
        }
    }

    pub async fn delete_videos_of_post<'c, A>(
        &self,
        video_path: &Path,
        acquirer: A,
        post_id: i64,
    ) -> Result<()>
    where
        A: Acquire<'c, Database = Sqlite>,
    {
        let mut conn = acquirer.acquire().await?;
        let video_paths = video::get_video_paths_by_post_id(&mut *conn, post_id).await?;
        video::delete_videos_by_post_id(&mut *conn, post_id).await?;

        for path in video_paths {
            let absolute_path = video_path.join(path);
            if absolute_path.exists()
                && let Err(e) = tokio::fs::remove_file(&absolute_path).await
            {
                log::error!(
                    "Failed to delete video file {}: {}",
                    absolute_path.display(),
                    e
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod local_tests {
    use sqlx::SqlitePool;
    use tempfile::tempdir;

    use super::*;
    use crate::models::{Video, VideoMeta};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    fn create_test_video(url: &str) -> Video {
        Video {
            meta: VideoMeta::new(url, 42).unwrap(),
            blob: Bytes::from_static(b"test video data"),
        }
    }

    #[tokio::test]
    async fn test_save_video() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemVideoStorage;
        let video = create_test_video("http://example.com/original/test.mp4");

        let db = setup_db().await;
        let result = storage.save_video(temp_dir.path(), &db, &video).await;
        assert!(result.is_ok());

        let expected_path = temp_dir.path().join("example.com/original/test.mp4");
        assert!(expected_path.exists());
        let data = tokio::fs::read(expected_path).await.unwrap();
        assert_eq!(data, video.blob);
    }

    #[tokio::test]
    async fn test_get_video_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemVideoStorage;
        let video = create_test_video("http://example.com/test.mp4");

        let db = setup_db().await;
        storage
            .save_video(temp_dir.path(), &db, &video)
            .await
            .unwrap();

        let blob = storage
            .get_video_blob(
                temp_dir.path(),
                &db,
                &Url::parse("http://example.com/test.mp4").unwrap(),
            )
            .await
            .unwrap();
        assert!(blob.is_some());
        assert_eq!(blob.unwrap(), video.blob);
    }

    #[tokio::test]
    async fn test_get_non_existent_video_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemVideoStorage;

        let db = setup_db().await;
        let blob = storage
            .get_video_blob(
                temp_dir.path(),
                &db,
                &Url::parse("http://example.com/non-existent.mp4").unwrap(),
            )
            .await
            .unwrap();
        assert!(blob.is_none());
    }
}
