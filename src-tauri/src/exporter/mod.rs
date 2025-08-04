#![allow(async_fn_in_trait)]
use std::io::ErrorKind;
use std::ops::RangeInclusive;
use std::path::PathBuf;

use bytes::Bytes;
use futures::future::join_all;
use log::info;
use tokio::{
    fs::{DirBuilder, File},
    io::AsyncWriteExt,
    sync::mpsc::Sender,
};

use crate::error::{Error, Result};
use crate::message::Message;
use crate::models::{Picture, PictureDefinition};
use crate::utils::url_to_filename;
use std::convert::TryFrom;

pub trait Exporter: Send + Sync {
    async fn export_page(&self, page: HTMLPage, options: &ExportOptions) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct HTMLPage {
    pub html: String,
    pub pics: Vec<HTMLPicture>,
}

#[derive(Debug, Clone)]
pub struct ExporterImpl {
    msg_sender: Sender<Message>,
}

impl ExporterImpl {
    pub fn new(msg_sender: Sender<Message>) -> Self {
        Self { msg_sender }
    }
}

impl Exporter for ExporterImpl {
    async fn export_page(&self, page: HTMLPage, options: &ExportOptions) -> Result<()>
where {
        info!(
            "export {}.html to {}",
            options.export_task_name,
            options.export_path.display()
        );
        let mut dir_builder = DirBuilder::new();
        dir_builder.recursive(true);
        if !options.export_path.exists() {
            dir_builder.create(options.export_path.as_path()).await?
        } else if !options.export_path.is_dir() {
            return Err(std::io::Error::new(
                ErrorKind::AlreadyExists,
                "export folder is a already exist file",
            )
            .into());
        }
        let html_file_name = options.export_task_name.to_owned() + ".html";
        let resources_dir_name = options.export_task_name.to_owned() + "_files";

        let mut operating_path = options.export_path.to_owned();
        operating_path.push(html_file_name);
        let mut html_file = File::create(operating_path.as_path()).await?;
        html_file.write_all(page.html.as_bytes()).await?;
        operating_path.pop();
        operating_path.push(resources_dir_name);
        dir_builder.create(operating_path.as_path()).await?;
        let operating_path = operating_path.as_path();
        join_all(page.pics.into_iter().map(|pic| async move {
            let mut pic_file = File::create(operating_path.join(pic.file_name)).await?;
            pic_file.write_all(&pic.blob).await
        }))
        .await;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HTMLPicture {
    pub file_name: String,
    pub blob: Bytes,
}

impl TryFrom<Picture> for HTMLPicture {
    type Error = Error;

    fn try_from(value: Picture) -> Result<Self> {
        let url_str = value.meta.url();
        let file_name = url_to_filename(url_str)?;

        Ok(HTMLPicture {
            file_name,
            blob: value.blob,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub pic_quality: PictureDefinition,
    pub export_path: PathBuf,
    pub export_task_name: String,
    pub posts_per_html: u32,
    pub reverse: bool,
    pub range: RangeInclusive<u32>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            pic_quality: PictureDefinition::default(),
            export_path: PathBuf::from("."),
            export_task_name: "weiback_export.html".to_string(),
            posts_per_html: 1000,
            reverse: false,
            range: 0..=u32::MAX,
        }
    }
}

impl ExportOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pic_quality(mut self, quality: PictureDefinition) -> Self {
        self.pic_quality = quality;
        self
    }

    pub fn export_path(mut self, path: PathBuf) -> Self {
        self.export_path = path;
        self
    }

    pub fn export_task_name(mut self, name: String) -> Self {
        self.export_task_name = name;
        self
    }

    pub fn posts_per_html(mut self, count: u32) -> Self {
        self.posts_per_html = count;
        self
    }

    pub fn range(mut self, range: RangeInclusive<u32>) -> Self {
        self.range = range;
        self
    }

    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
}

#[cfg(test)]
mod exporter_test {
    use super::{ExportOptions, Exporter, ExporterImpl, HTMLPage, HTMLPicture};
    use tokio::sync::mpsc::channel;
    #[tokio::test]
    async fn export_page() {
        let pic_blob = std::fs::read("res/example.jpg").unwrap();
        let page = HTMLPage {
            html: "testtesttest".into(),
            pics: vec![HTMLPicture {
                file_name: "example.jpg".into(),
                blob: pic_blob.into(),
            }]
            .into_iter()
            .collect(),
        };

        let (tx, _) = channel(1);
        let exporter = ExporterImpl::new(tx);
        let options = ExportOptions::default()
            .export_task_name("test_task".into())
            .export_path("./export_page".into());
        exporter.export_page(page, &options).await.unwrap();
        std::fs::remove_dir_all("export_page").unwrap();
    }
}
