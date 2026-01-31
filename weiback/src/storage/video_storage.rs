use std::fs::create_dir_all;
use std::path::Path;

use bytes::Bytes;
use log::debug;
use sqlx::SqlitePool;
use url::Url;

use super::internal::video;
use crate::error::{Error, Result};
use crate::models::Video;
use crate::utils::url_to_path;

#[derive(Debug, Clone, Default)]
pub struct FileSystemVideoStorage;

impl FileSystemVideoStorage {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemVideoStorage {
    pub async fn get_video_blob(
        &self,
        video_path: &Path,
        db: &SqlitePool,
        url: &Url,
    ) -> Result<Option<Bytes>> {
        let Some(relative_path) = video::get_video_path(db, url).await? else {
            return Ok(None);
        };
        let absolute_path = video_path.join(relative_path);
        match tokio::fs::read(&absolute_path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save_video(
        &self,
        video_path: &Path,
        db: &SqlitePool,
        video: &Video,
    ) -> Result<()> {
        let url = video.meta.url();
        let relative_path = url_to_path(url);
        let absolute_path = video_path.join(relative_path.as_path());
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
        video::save_video_meta(db, url, video.meta.post_id, relative_path.as_path()).await?;
        debug!("video {} saved to {:?}", video.meta.url(), absolute_path);
        Ok(())
    }

    pub async fn video_saved(&self, video_path: &Path, db: &SqlitePool, url: &Url) -> Result<bool> {
        let Some(relative_path) = video::get_video_path(db, url).await? else {
            return Ok(false);
        };
        let absolute_path = video_path.join(relative_path);
        Ok(absolute_path.exists())
    }
}

#[cfg(test)]
mod local_tests {
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
