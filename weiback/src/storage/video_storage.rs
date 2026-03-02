//! This module provides file system storage and retrieval for videos,
//! integrating with the database to manage video metadata.
//! It handles saving, retrieving, and deleting video blobs,
//! and ensures consistency between file system presence and database records.

use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::{debug, warn};
use sqlx::{Acquire, Executor, Sqlite};
use url::Url;

use super::internal::video;
use crate::error::{Error, Result};
use crate::models::Video;
use crate::utils::livephoto_video_url_to_path_str;

/// A struct responsible for storing and retrieving video files on the file system.
/// It works in conjunction with the database to manage video metadata.
#[derive(Debug, Clone, Default)]
pub struct FileSystemVideoStorage;

impl FileSystemVideoStorage {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemVideoStorage {
    /// Retrieves the binary content (blob) of a video from the file system.
    ///
    /// If the video file is not found on disk but an entry exists in the database,
    /// the database entry will be deleted.
    ///
    /// # Arguments
    ///
    /// * `video_path` - The base directory where videos are stored.
    /// * `acquirer` - A database acquirer.
    /// * `url` - The URL of the video to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option<Bytes>`. `Some(Bytes)` if the video is found, `None` otherwise.
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

    /// Saves a video's binary content to the file system and its metadata to the database.
    ///
    /// Creates necessary parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `video_path` - The base directory where videos should be stored.
    /// * `executor` - A database executor.
    /// * `video` - The `Video` object containing metadata and binary blob.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
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
        let relative_path = PathBuf::from(livephoto_video_url_to_path_str(url)?);
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

    /// Checks if a video is saved (both in the database and on the file system).
    ///
    /// If the video is found in the database but not on the file system, its database entry will be deleted.
    ///
    /// # Arguments
    ///
    /// * `video_path` - The base directory where videos are stored.
    /// * `acquirer` - A database acquirer.
    /// * `url` - The URL of the video to check.
    ///
    /// # Returns
    ///
    /// A `Result` containing `true` if the video is saved, `false` otherwise.
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

    /// Deletes all videos associated with a given post from both the file system and the database.
    ///
    /// # Arguments
    ///
    /// * `video_path` - The base directory where videos are stored.
    /// * `acquirer` - A database acquirer.
    /// * `post_id` - The ID of the post whose videos are to be deleted.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
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
        let video = create_test_video(
            "https://video.weibo.com/media/play?livephoto=https%3A%2F%2Fus.sinaimg.cn%2F0023jbLigx081byvTnCw0f0f01004O5e0k01.mov",
        );

        let db = setup_db().await;
        let result = storage.save_video(temp_dir.path(), &db, &video).await;
        assert!(result.is_ok());

        let expected_path = temp_dir
            .path()
            .join("us.sinaimg.cn/0023jbLigx081byvTnCw0f0f01004O5e0k01.mov");
        assert!(expected_path.exists());
        let data = tokio::fs::read(expected_path).await.unwrap();
        assert_eq!(data, video.blob);
    }

    #[tokio::test]
    async fn test_get_video_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemVideoStorage;
        let video = create_test_video(
            "https://video.weibo.com/media/play?livephoto=https%3A%2F%2Fus.sinaimg.cn%2F0023jbLigx081byvTnCw0f0f01004O5e0k01.mov",
        );

        let db = setup_db().await;
        storage
            .save_video(temp_dir.path(), &db, &video)
            .await
            .unwrap();

        let blob = storage
            .get_video_blob(
                temp_dir.path(),
                &db,
                &Url::parse("https://video.weibo.com/media/play?livephoto=https%3A%2F%2Fus.sinaimg.cn%2F0023jbLigx081byvTnCw0f0f01004O5e0k01.mov").unwrap(),
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
