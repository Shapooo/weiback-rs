//! This module provides file system storage and retrieval for pictures,
//! integrating with the database to manage picture metadata.
//! It handles saving, retrieving, and deleting picture blobs,
//! and ensures consistency between file system presence and database records.

use std::fs::create_dir_all;
use std::path::Path;

use bytes::Bytes;
use sqlx::{Acquire, Executor, Sqlite};
use tracing::{debug, error, warn};
use url::Url;

use super::internal::picture;
use crate::error::{Error, Result};
use crate::models::Picture;
use crate::utils::pic_url_to_path_str;

/// A struct responsible for storing and retrieving picture files on the file system.
/// It works in conjunction with the database to manage picture metadata.
#[derive(Debug, Clone, Default)]
pub struct FileSystemPictureStorage;

impl FileSystemPictureStorage {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemPictureStorage {
    /// Retrieves the binary content (blob) of a picture from the file system.
    ///
    /// # Arguments
    ///
    /// * `picture_path` - The base directory where pictures are stored.
    /// * `executor` - A database executor.
    /// * `url` - The URL of the picture to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option<Bytes>`. `Some(Bytes)` if the picture is found, `None` otherwise.
    pub async fn get_picture_blob<'e, E>(
        &self,
        picture_path: &Path,
        executor: E,
        url: &Url,
    ) -> Result<Option<Bytes>>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let Some(relative_path) = picture::get_picture_path(executor, url).await? else {
            return Ok(None);
        };
        let path = picture_path.join(&relative_path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!("picture has db entry, but file not found at {:?}", path);
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Saves a picture's binary content to the file system and its metadata to the database.
    ///
    /// Creates necessary parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `picture_path` - The base directory where pictures should be stored.
    /// * `executor` - A database executor.
    /// * `picture` - The `Picture` object containing metadata and binary blob.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub async fn save_picture<'e, E>(
        &self,
        picture_path: &Path,
        executor: E,
        picture: &Picture,
    ) -> Result<()>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let url = picture.meta.url();
        let relative_path = pic_url_to_path_str(url);
        let absolute_path = picture_path.join(&relative_path);
        create_dir_all(
            absolute_path
                .parent()
                .ok_or(Error::Io(std::io::Error::other(
                    "cannot get parent of picture path",
                )))?,
        )?;
        if let Some(parent) = absolute_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&absolute_path, &picture.blob).await?;
        picture::save_picture_meta(executor, &picture.meta, Some(relative_path.as_str())).await?;
        debug!(
            "picture {} saved to {:?}",
            picture.meta.url(),
            absolute_path
        );
        Ok(())
    }

    /// Checks if a picture is saved (both in the database and on the file system).
    ///
    /// # Arguments
    ///
    /// * `picture_path` - The base directory where pictures are stored.
    /// * `executor` - A database executor.
    /// * `url` - The URL of the picture to check.
    ///
    /// # Returns
    ///
    /// A `Result` containing `true` if the picture is saved, `false` otherwise.
    pub async fn picture_saved<'e, E>(
        &self,
        picture_path: &Path,
        executor: E,
        url: &Url,
    ) -> Result<bool>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let Some(relative_path) = picture::get_picture_path(executor, url).await? else {
            return Ok(false);
        };
        let absolute_path = picture_path.join(relative_path);
        if absolute_path.exists() {
            Ok(true)
        } else {
            warn!(
                "picture has db entry, but file not found at {:?}",
                absolute_path
            );
            Ok(false)
        }
    }

    /// Deletes a specific picture from both the file system and the database.
    ///
    /// # Arguments
    ///
    /// * `picture_path` - The base directory where pictures are stored.
    /// * `acquirer` - A database acquirer.
    /// * `url` - The URL of the picture to delete.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub async fn delete_picture<'c, A>(
        &self,
        picture_path: &Path,
        acquirer: A,
        url: &Url,
    ) -> Result<()>
    where
        A: Acquire<'c, Database = Sqlite>,
    {
        let mut conn = acquirer.acquire().await?;
        if let Some(relative_path) = picture::get_picture_path(&mut *conn, url).await? {
            let absolute_path = picture_path.join(relative_path);
            if absolute_path.exists() {
                tokio::fs::remove_file(&absolute_path).await?;
            }
            picture::delete_picture_by_url(&mut *conn, url).await?;
        }
        Ok(())
    }

    /// Deletes all pictures associated with a given post from both the file system and the database.
    ///
    /// # Arguments
    ///
    /// * `picture_path` - The base directory where pictures are stored.
    /// * `db` - The database connection pool.
    /// * `post_id` - The ID of the post whose pictures are to be deleted.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub async fn delete_post_pictures<'c, A>(
        &self,
        picture_path: &Path,
        acquirer: A,
        post_id: i64,
    ) -> Result<()>
    where
        A: Acquire<'c, Database = Sqlite>,
    {
        self.batch_delete_posts_pictures(picture_path, acquirer, &[post_id])
            .await
    }

    /// Deletes all pictures associated with a given list of posts from both the file system and the database.
    ///
    /// # Arguments
    ///
    /// * `picture_path` - The base directory where pictures are stored.
    /// * `acquirer` - A database acquirer.
    /// * `post_ids` - A slice of IDs of the posts whose pictures are to be deleted.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub async fn batch_delete_posts_pictures<'c, A>(
        &self,
        picture_path: &Path,
        acquirer: A,
        post_ids: &[i64],
    ) -> Result<()>
    where
        A: Acquire<'c, Database = Sqlite>,
    {
        let mut conn = acquirer.acquire().await?;
        let pic_infos = picture::get_pictures_by_post_ids(&mut *conn, post_ids).await?;
        picture::delete_pictures_by_post_ids(&mut *conn, post_ids).await?;

        for info in pic_infos {
            let pic_path = picture_path.join(info.path);
            if pic_path.exists()
                && let Err(e) = tokio::fs::remove_file(&pic_path).await
            {
                error!(
                    "Failed to delete picture file {}: {}",
                    pic_path.display(),
                    e
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod local_tests {
    use tempfile::tempdir;

    use sqlx::SqlitePool;

    use super::*;
    use crate::models::{Picture, PictureDefinition, PictureMeta};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    fn create_test_picture(url: &str) -> Picture {
        Picture {
            meta: PictureMeta::attached(url, 42, PictureDefinition::Largest).unwrap(),
            blob: Bytes::from_static(b"test picture data"),
        }
    }

    #[tokio::test]
    async fn test_save_picture() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let picture = create_test_picture("http://example.com/original/test.jpg");

        let db = setup_db().await;
        let result = storage.save_picture(temp_dir.path(), &db, &picture).await;
        assert!(result.is_ok());

        let expected_path = temp_dir.path().join("example.com/original/test.jpg");
        assert!(expected_path.exists());
        let data = tokio::fs::read(expected_path).await.unwrap();
        assert_eq!(data, picture.blob);
    }

    #[tokio::test]
    async fn test_get_picture_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let picture = create_test_picture("http://example.com/test.jpg");

        let db = setup_db().await;
        storage
            .save_picture(temp_dir.path(), &db, &picture)
            .await
            .unwrap();

        let blob = storage
            .get_picture_blob(
                temp_dir.path(),
                &db,
                &Url::parse("http://example.com/test.jpg").unwrap(),
            )
            .await
            .unwrap();
        assert!(blob.is_some());
        assert_eq!(blob.unwrap(), picture.blob);
    }

    #[tokio::test]
    async fn test_get_non_existent_picture_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;

        let db = setup_db().await;
        let blob = storage
            .get_picture_blob(
                temp_dir.path(),
                &db,
                &Url::parse("http://example.com/non-existent.jpg").unwrap(),
            )
            .await
            .unwrap();
        assert!(blob.is_none());
    }

    #[tokio::test]
    async fn test_picture_saved_exists() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let picture = create_test_picture("http://example.com/exists.jpg");

        let db = setup_db().await;
        storage
            .save_picture(temp_dir.path(), &db, &picture)
            .await
            .unwrap();

        let saved = storage
            .picture_saved(
                temp_dir.path(),
                &db,
                &Url::parse("http://example.com/exists.jpg").unwrap(),
            )
            .await
            .unwrap();
        assert!(saved);
    }

    #[tokio::test]
    async fn test_picture_saved_db_only() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let picture = create_test_picture("http://example.com/db_only.jpg");

        let db = setup_db().await;
        // Save to DB only, not to file system (by manually calling picture::save_picture_meta)
        let url = picture.meta.url().clone();
        let relative_path = pic_url_to_path_str(&url);
        picture::save_picture_meta(&db, &picture.meta, Some(relative_path.as_str()))
            .await
            .unwrap();

        assert!(!temp_dir.path().join("example.com/db_only.jpg").exists()); // Ensure file does not exist

        let saved = storage
            .picture_saved(temp_dir.path(), &db, &url)
            .await
            .unwrap();
        assert!(!saved); // Should return false because file doesn't exist
    }

    #[tokio::test]
    async fn test_delete_picture() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let picture = create_test_picture("http://example.com/todelete.jpg");

        let db = setup_db().await;
        storage
            .save_picture(temp_dir.path(), &db, &picture)
            .await
            .unwrap();

        let url = picture.meta.url().clone();
        let file_path = temp_dir.path().join(pic_url_to_path_str(&url));
        assert!(file_path.exists());
        assert!(
            picture::get_picture_path(&db, &url)
                .await
                .unwrap()
                .is_some()
        );

        storage
            .delete_picture(temp_dir.path(), &db, &url)
            .await
            .unwrap();

        assert!(!file_path.exists());
        assert!(
            picture::get_picture_path(&db, &url)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_pictures_of_post() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let post_id = 123;

        let pic1 = Picture {
            meta: PictureMeta::attached(
                "http://example.com/post/pic1.jpg",
                post_id,
                PictureDefinition::Largest,
            )
            .unwrap(),
            blob: Bytes::from_static(b"pic1 data"),
        };
        let pic2 = Picture {
            meta: PictureMeta::attached(
                "http://example.com/post/pic2.jpg",
                post_id,
                PictureDefinition::Largest,
            )
            .unwrap(),
            blob: Bytes::from_static(b"pic2 data"),
        };
        let unrelated_pic = Picture {
            meta: PictureMeta::attached(
                "http://example.com/unrelated.jpg",
                999,
                PictureDefinition::Largest,
            )
            .unwrap(),
            blob: Bytes::from_static(b"unrelated data"),
        };

        let db = setup_db().await;
        storage
            .save_picture(temp_dir.path(), &db, &pic1)
            .await
            .unwrap();
        storage
            .save_picture(temp_dir.path(), &db, &pic2)
            .await
            .unwrap();
        storage
            .save_picture(temp_dir.path(), &db, &unrelated_pic)
            .await
            .unwrap();

        let file_path1 = temp_dir.path().join(pic_url_to_path_str(pic1.meta.url()));
        let file_path2 = temp_dir.path().join(pic_url_to_path_str(pic2.meta.url()));
        let unrelated_file_path = temp_dir
            .path()
            .join(pic_url_to_path_str(unrelated_pic.meta.url()));

        assert!(file_path1.exists());
        assert!(file_path2.exists());
        assert!(unrelated_file_path.exists());
        assert!(
            picture::get_pictures_by_post_id(&db, post_id)
                .await
                .unwrap()
                .len()
                == 2
        );

        storage
            .delete_post_pictures(temp_dir.path(), &db, post_id)
            .await
            .unwrap();

        assert!(!file_path1.exists());
        assert!(!file_path2.exists());
        assert!(unrelated_file_path.exists()); // Unrelated picture should still exist
        assert!(
            picture::get_pictures_by_post_id(&db, post_id)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            picture::get_picture_path(&db, unrelated_pic.meta.url())
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_batch_delete_post_pictures() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSystemPictureStorage;
        let post_id1 = 1;
        let post_id2 = 2;

        let pic1 = Picture {
            meta: PictureMeta::attached(
                "http://example.com/post1/pic1.jpg",
                post_id1,
                PictureDefinition::Largest,
            )
            .unwrap(),
            blob: Bytes::from_static(b"pic1 data"),
        };
        let pic2 = Picture {
            meta: PictureMeta::attached(
                "http://example.com/post2/pic2.jpg",
                post_id2,
                PictureDefinition::Largest,
            )
            .unwrap(),
            blob: Bytes::from_static(b"pic2 data"),
        };

        let db = setup_db().await;
        storage
            .save_picture(temp_dir.path(), &db, &pic1)
            .await
            .unwrap();
        storage
            .save_picture(temp_dir.path(), &db, &pic2)
            .await
            .unwrap();

        let file_path1 = temp_dir.path().join(pic_url_to_path_str(pic1.meta.url()));
        let file_path2 = temp_dir.path().join(pic_url_to_path_str(pic2.meta.url()));

        assert!(file_path1.exists());
        assert!(file_path2.exists());

        storage
            .batch_delete_posts_pictures(temp_dir.path(), &db, &[post_id1, post_id2])
            .await
            .unwrap();

        assert!(!file_path1.exists());
        assert!(!file_path2.exists());
        assert!(
            picture::get_pictures_by_post_id(&db, post_id1)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            picture::get_pictures_by_post_id(&db, post_id2)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
