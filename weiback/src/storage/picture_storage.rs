use std::env::current_exe;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::debug;

use crate::config::get_config;
use crate::error::Result;
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

        let picture_path = current_exe()?.parent().unwrap().join(picture_path);
        Ok(FileSystemPictureStorage { picture_path })
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
        let path = url_to_path(picture.meta.url())?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap(); // promised to start with '/'
        let path = self.picture_path.join(relative_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &picture.blob).await?;
        debug!("picture {} saved to {:?}", picture.meta.url(), path);
        Ok(())
    }
}
