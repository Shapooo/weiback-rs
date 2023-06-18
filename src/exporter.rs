use std::path::{Path, PathBuf};

use anyhow;
use bytes::Bytes;
use tokio::fs::{DirBuilder, File};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct Exporter();

impl Exporter {
    pub fn new() -> Self {
        Exporter()
    }

    pub async fn export_page<N, P>(
        &self,
        task_name: N,
        page: HTMLPage,
        path: P,
    ) -> anyhow::Result<()>
    where
        N: AsRef<str>,
        P: AsRef<Path>,
    {
        // TODO: handle condition files and folders exist
        if !path.as_ref().is_dir() {
            return Err(anyhow::anyhow!("export path is not a valid dir"));
        }
        let mut operating_path = PathBuf::from(path.as_ref());
        operating_path.push(task_name.as_ref());
        let dir_builder = DirBuilder::new();
        dir_builder.create(operating_path.as_path()).await?;
        operating_path.push("weiback.html");
        let mut html_file = File::create(operating_path.as_path()).await?;
        html_file.write_all(page.html.as_bytes()).await?;
        operating_path.pop();
        operating_path.push("weiback_files");
        dir_builder.create(operating_path.as_path()).await?;
        for pic in page.pics.into_iter() {
            operating_path.push(pic.name);
            let mut pic_file = File::create(operating_path.as_path()).await?;
            pic_file.write_all(&pic.blob).await?;
            operating_path.pop();
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HTMLPage {
    pub html: String,
    pub pics: Vec<Picture>,
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub name: String,
    pub blob: Bytes,
}

#[cfg(test)]
mod exporter_test {
    use super::{Exporter, HTMLPage, Picture};
    #[tokio::test]
    async fn export_page() {
        let e = Exporter::new();
        let pic_blob = std::fs::read("res/example.jpg").unwrap();
        let page = HTMLPage {
            html: "testtesttest".into(),
            pics: vec![Picture {
                name: "example.jpg".into(),
                blob: pic_blob.into(),
            }]
            .into_iter()
            .collect(),
        };
        e.export_page("test_task", page, ".").await.unwrap();
        std::fs::remove_dir_all("test_task").unwrap();
    }
}
