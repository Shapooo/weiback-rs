use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::debug;

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
    pub async fn get_picture_blob(&self, url: &str) -> Result<Option<Bytes>> {
        let path = url_to_path(url)?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap(); // promised to start with '/'
        let path = self.picture_path.join(relative_path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save_picture(&self, picture: &Picture) -> Result<()> {
        let url = picture.meta.url();
        let path = url_to_path(url)?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap(); // promised to start with '/'
        let path = self.picture_path.join(relative_path);
        create_dir_all(path.parent().ok_or(Error::Io(std::io::Error::other(
            "cannot get parent of picture path",
        )))?)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &picture.blob).await?;
        debug!("picture {} saved to {:?}", picture.meta.url(), path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Picture, PictureMeta};
    use tempfile::tempdir;

    fn create_test_storage(temp_dir: &Path) -> FileSystemPictureStorage {
        FileSystemPictureStorage {
            picture_path: temp_dir.to_path_buf(),
        }
    }

    fn create_test_picture(url: &str) -> Picture {
        Picture {
            meta: PictureMeta::in_post(url.to_string(), 42),
            blob: Bytes::from_static(b"test picture data"),
        }
    }

    #[tokio::test]
    async fn test_save_picture() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());
        let picture = create_test_picture("http://example.com/original/test.jpg");

        let result = storage.save_picture(&picture).await;
        assert!(result.is_ok());

        let expected_path = temp_dir.path().join("original").join("test.jpg");
        assert!(expected_path.exists());
        let data = tokio::fs::read(expected_path).await.unwrap();
        assert_eq!(data, picture.blob);
    }

    #[tokio::test]
    async fn test_get_picture_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());
        let picture = create_test_picture("http://example.com/test.jpg");

        storage.save_picture(&picture).await.unwrap();

        let blob = storage
            .get_picture_blob("http://example.com/test.jpg")
            .await
            .unwrap();
        assert!(blob.is_some());
        assert_eq!(blob.unwrap(), picture.blob);
    }

    #[tokio::test]
    async fn test_get_non_existent_picture_blob() {
        let temp_dir = tempdir().unwrap();
        let storage = create_test_storage(temp_dir.path());

        let blob = storage
            .get_picture_blob("http://example.com/non-existent.jpg")
            .await
            .unwrap();
        assert!(blob.is_none());
    }
}
