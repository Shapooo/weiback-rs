use std::fs::create_dir_all;
use std::path::PathBuf;

use bytes::Bytes;
use log::debug;
use sqlx::SqlitePool;
use url::Url;

use super::internal::picture;
use crate::config::get_config;
use crate::error::{Error, Result};
use crate::models::Picture;
use crate::utils::url_to_path;

#[derive(Debug, Clone)]
pub struct FileSystemPictureStorage {
    picture_path: PathBuf,
}

impl FileSystemPictureStorage {
    pub fn new() -> Result<Self> {
        let config = get_config();
        let config_read = config.read()?;
        let picture_path = config_read.picture_path.clone();
        drop(config_read);

        Ok(FileSystemPictureStorage { picture_path })
    }

    #[cfg(test)]
    pub fn from_picture_path(picture_path: PathBuf) -> Self {
        Self { picture_path }
    }
}

impl FileSystemPictureStorage {
    pub async fn get_picture_blob(&self, db: &SqlitePool, url: &Url) -> Result<Option<Bytes>> {
        let Some(path) = picture::get_picture_path(db, url).await? else {
            return Ok(None);
        };
        let path = self.picture_path.join(path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save_picture(&self, db: &SqlitePool, picture: &Picture) -> Result<()> {
        let url = picture.meta.url();
        let relative_path = url_to_path(url);
        let absolute_path = self.picture_path.join(relative_path.as_path());
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
        picture::save_picture_meta(db, &picture.meta, Some(relative_path.as_path())).await?;
        debug!(
            "picture {} saved to {:?}",
            picture.meta.url(),
            absolute_path
        );
        Ok(())
    }

    pub async fn picture_saved(&self, db: &SqlitePool, url: &Url) -> Result<bool> {
        let Some(relative_path) = picture::get_picture_path(db, url).await? else {
            return Ok(false);
        };
        let absolute_path = self.picture_path.join(relative_path);
        Ok(absolute_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::models::{Picture, PictureMeta};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        super::super::internal::picture::create_picture_table(&pool)
            .await
            .unwrap();
        pool
    }

    fn create_test_storage(temp_dir: &Path) -> FileSystemPictureStorage {
        FileSystemPictureStorage {
            picture_path: temp_dir.to_path_buf(),
        }
    }

    fn create_test_picture(url: &str) -> Picture {
        Picture {
            meta: PictureMeta::in_post(url, 42).unwrap(),
            blob: Bytes::from_static(b"test picture data"),
        }
    }

    #[tokio::test]
    async fn test_save_picture() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());
        let picture = create_test_picture("http://example.com/original/test.jpg");

        let db = setup_db().await;
        let result = storage.save_picture(&db, &picture).await;
        assert!(result.is_ok());

        let expected_path = temp_dir.path().join("example.com/original/test.jpg");
        assert!(expected_path.exists());
        let data = tokio::fs::read(expected_path).await.unwrap();
        assert_eq!(data, picture.blob);
    }

    #[tokio::test]
    async fn test_get_picture_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());
        let picture = create_test_picture("http://example.com/test.jpg");

        let db = setup_db().await;
        storage.save_picture(&db, &picture).await.unwrap();

        let blob = storage
            .get_picture_blob(&db, &Url::parse("http://example.com/test.jpg").unwrap())
            .await
            .unwrap();
        assert!(blob.is_some());
        assert_eq!(blob.unwrap(), picture.blob);
    }

    #[tokio::test]
    async fn test_get_non_existent_picture_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());

        let db = setup_db().await;
        let blob = storage
            .get_picture_blob(
                &db,
                &Url::parse("http://example.com/non-existent.jpg").unwrap(),
            )
            .await
            .unwrap();
        assert!(blob.is_none());
    }
}
