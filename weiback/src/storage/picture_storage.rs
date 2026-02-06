use std::fs::create_dir_all;
use std::path::Path;

use bytes::Bytes;
use log::debug;
use sqlx::{Executor, Sqlite};
use url::Url;

use super::internal::picture;
use crate::error::{Error, Result};
use crate::models::Picture;
use crate::utils::url_to_path;

#[derive(Debug, Clone, Default)]
pub struct FileSystemPictureStorage;

impl FileSystemPictureStorage {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemPictureStorage {
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
        let path = picture_path.join(relative_path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

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
        let relative_path = url_to_path(url);
        let absolute_path = picture_path.join(relative_path.as_path());
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
        picture::save_picture_meta(executor, &picture.meta, Some(relative_path.as_path())).await?;
        debug!(
            "picture {} saved to {:?}",
            picture.meta.url(),
            absolute_path
        );
        Ok(())
    }

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
        Ok(absolute_path.exists())
    }

    pub async fn delete_pictures_of_post(
        &self,
        picture_path: &Path,
        db: &sqlx::SqlitePool,
        post_id: i64,
    ) -> Result<()> {
        let pic_infos = picture::get_pictures_by_post_id(db, post_id).await?;
        picture::delete_pictures_by_post_id(db, post_id).await?;

        for info in pic_infos {
            let pic_path = picture_path.join(info.path);
            if pic_path.exists()
                && let Err(e) = tokio::fs::remove_file(&pic_path).await
            {
                log::error!(
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
            meta: PictureMeta::in_post(url, PictureDefinition::Largest, 42).unwrap(),
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
}
