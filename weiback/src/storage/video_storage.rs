use std::fs::create_dir_all;
use std::path::PathBuf;

use bytes::Bytes;
use log::debug;
use sqlx::SqlitePool;
use url::Url;

use super::internal::video;
use crate::config::get_config;
use crate::error::{Error, Result};
use crate::models::Video;
use crate::utils::url_to_path;

#[derive(Debug, Clone)]
pub struct FileSystemVideoStorage {
    video_path: PathBuf,
}

impl FileSystemVideoStorage {
    pub fn new() -> Result<Self> {
        let config = get_config();
        let config_read = config.read()?;
        let video_path = config_read.video_path.clone();
        drop(config_read);

        Ok(FileSystemVideoStorage { video_path })
    }

    #[cfg(test)]
    pub fn from_video_path(video_path: PathBuf) -> Self {
        Self { video_path }
    }
}

impl FileSystemVideoStorage {
    pub async fn get_video_blob(&self, db: &SqlitePool, url: &Url) -> Result<Option<Bytes>> {
        let Some(path) = video::get_video_path(db, url).await? else {
            return Ok(None);
        };
        let path = self.video_path.join(path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save_video(&self, db: &SqlitePool, video: &Video) -> Result<()> {
        let url = video.meta.url();
        let relative_path = url_to_path(url);
        let absolute_path = self.video_path.join(relative_path.as_path());
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

    pub async fn video_saved(&self, db: &SqlitePool, url: &Url) -> Result<bool> {
        let Some(relative_path) = video::get_video_path(db, url).await? else {
            return Ok(false);
        };
        let absolute_path = self.video_path.join(relative_path);
        Ok(absolute_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::models::{Video, VideoMeta};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    fn create_test_storage(temp_dir: &Path) -> FileSystemVideoStorage {
        FileSystemVideoStorage {
            video_path: temp_dir.to_path_buf(),
        }
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
        let storage = create_test_storage(temp_dir.path());
        let video = create_test_video("http://example.com/original/test.mp4");

        let db = setup_db().await;
        let result = storage.save_video(&db, &video).await;
        assert!(result.is_ok());

        let expected_path = temp_dir.path().join("example.com/original/test.mp4");
        assert!(expected_path.exists());
        let data = tokio::fs::read(expected_path).await.unwrap();
        assert_eq!(data, video.blob);
    }

    #[tokio::test]
    async fn test_get_video_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());
        let video = create_test_video("http://example.com/test.mp4");

        let db = setup_db().await;
        storage.save_video(&db, &video).await.unwrap();

        let blob = storage
            .get_video_blob(&db, &Url::parse("http://example.com/test.mp4").unwrap())
            .await
            .unwrap();
        assert!(blob.is_some());
        assert_eq!(blob.unwrap(), video.blob);
    }

    #[tokio::test]
    async fn test_get_non_existent_video_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());

        let db = setup_db().await;
        let blob = storage
            .get_video_blob(
                &db,
                &Url::parse("http://example.com/non-existent.mp4").unwrap(),
            )
            .await
            .unwrap();
        assert!(blob.is_none());
    }
}
