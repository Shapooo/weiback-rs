use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

use anyhow::Result;
use futures::future::join_all;
use log::info;
use tokio::fs::{DirBuilder, File};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct Exporter();

impl Exporter {
    pub async fn export_page<N, P>(html_name: N, page: HTMLPage, path: P) -> Result<()>
    where
        N: AsRef<str>,
        P: AsRef<Path>,
    {
        info!(
            "export {}.html to {}",
            html_name.as_ref(),
            path.as_ref().display()
        );
        let mut dir_builder = DirBuilder::new();
        dir_builder.recursive(true);
        if !path.as_ref().exists() {
            dir_builder.create(path.as_ref()).await?
        } else if !path.as_ref().is_dir() {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "export folder is a already exist file",
            )
            .into());
        }
        let html_file_name = String::from(html_name.as_ref()) + ".html";
        let resources_dir_name = String::from(html_name.as_ref()) + "_files";

        let mut operating_path = PathBuf::from(path.as_ref());
        operating_path.push(html_file_name);
        let mut html_file = File::create(operating_path.as_path()).await?;
        html_file.write_all(page.html.as_bytes()).await?;
        operating_path.pop();
        operating_path.push(resources_dir_name);
        dir_builder.create(operating_path.as_path()).await?;
        join_all(page.pics.into_iter().map(|pic| async {
            let pic = pic;
            let mut pic_file = File::create(operating_path.join(pic.name)).await?;
            pic_file.write_all(&pic.blob).await
        }))
        .await;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HTMLPage {
    pub html: String,
    pub pics: Vec<HTMLPicture>,
}

#[derive(Debug, Clone)]
pub struct HTMLPicture {
    pub name: String,
    pub blob: Vec<u8>,
}

#[cfg(test)]
mod exporter_test {
    use super::{Exporter, HTMLPage, HTMLPicture};
    #[tokio::test]
    async fn export_page() {
        let pic_blob = std::fs::read("res/example.jpg").unwrap();
        let page = HTMLPage {
            html: "testtesttest".into(),
            pics: vec![HTMLPicture {
                name: "example.jpg".into(),
                blob: pic_blob.into(),
            }]
            .into_iter()
            .collect(),
        };
        Exporter::export_page("test_task", page, "./export_page")
            .await
            .unwrap();
        std::fs::remove_dir_all("export_page").unwrap();
    }
}
