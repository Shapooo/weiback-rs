//! This module provides the `Exporter` trait and its implementation for writing
//! generated HTML and associated media files to the local file system.
//!
//! The main entry point is the `Exporter` trait, which defines the `export_page`
//! method. `ExporterImpl` provides the concrete logic for creating directories,
//! writing the HTML file, and copying all necessary image files from the local
//! cache to a resource sub-folder.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use tokio::{
    fs::{DirBuilder, File},
    io::AsyncWriteExt,
};
use tracing::{debug, error, info, warn};

use crate::error::Result;
use crate::utils::{make_html_file_name, make_resource_dir_name};

/// A trait for services that can export an `HTMLPage` to a persistent medium.
#[async_trait]
pub trait Exporter: Send + Sync {
    /// Exports a single HTML page and its associated pictures.
    ///
    /// # Arguments
    /// * `page` - The `HTMLPage` containing the HTML content and pictures to export.
    /// * `page_name` - The base name for the output file (e.g., "page-1").
    /// * `export_dir` - The top-level directory where the export should be saved.
    async fn export_page(&self, page: HTMLPage, page_name: &str, export_dir: &Path) -> Result<()>;
}

/// Represents a single picture to be exported.
#[derive(Debug, Clone)]
pub struct PictureExport {
    /// The path to the source image file in the local cache.
    pub source_path: PathBuf,
    /// The target filename for the image in the export's resource directory.
    pub target_file_name: String,
}

/// A container for the complete data needed to render and export a single HTML page.
#[derive(Debug, Clone)]
pub struct HTMLPage {
    /// The fully rendered HTML content.
    pub html: String,
    /// A list of all pictures referenced in the HTML that need to be copied to the export directory.
    pub pictures_to_export: Vec<PictureExport>,
}

/// The default implementation of the `Exporter` trait, which writes to the local file system.
#[derive(Debug, Clone, Default)]
pub struct ExporterImpl();

impl ExporterImpl {
    /// Creates a new `ExporterImpl`.
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait]
impl Exporter for ExporterImpl {
    /// Writes the HTML content to a file and copies all associated pictures to a
    /// corresponding `_files` directory.
    ///
    /// For a `page_name` of "my-export-1", this would produce:
    /// - `export_dir/my-export-1.html`
    /// - `export_dir/my-export-1_files/` (containing all images)
    ///
    /// This method is idempotent and will overwrite existing files.
    ///
    /// # Errors
    /// Returns an `Error::Io` if the `export_dir` path exists but is a file, or if
    /// there are issues with file I/O operations.
    async fn export_page(&self, page: HTMLPage, page_name: &str, export_dir: &Path) -> Result<()>
where {
        info!("Exporting page for task '{page_name}' to {export_dir:?}",);
        let mut dir_builder = DirBuilder::new();
        dir_builder.recursive(true);
        if !export_dir.exists() {
            debug!("Creating export directory at {export_dir:?}",);
            dir_builder.create(export_dir).await?
        } else if !export_dir.is_dir() {
            error!("Export path {} is not a directory", export_dir.display());
            return Err(std::io::Error::new(
                ErrorKind::AlreadyExists,
                "export folder is a already exist file",
            )
            .into());
        }
        let html_file_name = make_html_file_name(page_name);
        let html_file_path = export_dir.join(html_file_name);
        debug!("Writing HTML to file: {html_file_path:?}");
        let mut html_file = File::create(&html_file_path).await?;
        html_file.write_all(page.html.as_bytes()).await?;
        debug!("Successfully wrote HTML to {html_file_path:?}");

        let resources_dir_name = make_resource_dir_name(page_name);
        let resources_dir_path = export_dir.join(resources_dir_name);
        if !resources_dir_path.exists() && !page.pictures_to_export.is_empty() {
            dir_builder.create(&resources_dir_path).await?;
        }
        let pic_output_dir = resources_dir_path.as_path();
        debug!(
            "Copying {} picture files to {:?}",
            page.pictures_to_export.len(),
            pic_output_dir
        );
        let pic_futures = page.pictures_to_export.into_iter().map(|pic| async move {
            let dest_path = pic_output_dir.join(pic.target_file_name.clone());
            tokio::fs::copy(&pic.source_path, &dest_path)
                .await
                .map_err(|e| {
                    error!(
                        "Failed to copy picture from {:?} to {:?}: {}",
                        pic.source_path, dest_path, e
                    );
                    e
                })?;
            Result::<_>::Ok(())
        });
        let fail_sum = stream::iter(pic_futures)
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter(|r| r.is_err())
            .count();
        if fail_sum > 0 {
            warn!("{fail_sum} pictures exports failed");
        }
        info!("Finished exporting page for task '{page_name}'");
        Ok(())
    }
}

#[cfg(test)]
mod local_tests {
    use tempfile::{TempDir, tempdir};
    use tokio::fs;

    use super::*;
    use crate::error::Error;

    // New test helper to create source files
    async fn create_test_page_with_files(
        temp_dir: &TempDir,
        html_content: &str,
        num_pics: usize,
    ) -> (HTMLPage, Vec<String>) {
        let source_dir = temp_dir.path().join("source_pics");
        fs::create_dir_all(&source_dir).await.unwrap();

        let mut expected_pic_contents = Vec::new();

        let pictures_to_export = (0..num_pics)
            .map(|i| {
                let source_file_name = format!("source_{}.jpg", i);
                let source_path = source_dir.join(&source_file_name);
                let content = format!("pic_content_{}", i);
                std::fs::write(&source_path, &content).unwrap();
                expected_pic_contents.push(content);

                PictureExport {
                    source_path,
                    target_file_name: format!("target_{}.jpg", i),
                }
            })
            .collect();

        (
            HTMLPage {
                html: html_content.to_string(),
                pictures_to_export,
            },
            expected_pic_contents,
        )
    }

    // A simpler helper for tests without pictures
    fn create_test_page_no_files(html_content: &str) -> HTMLPage {
        HTMLPage {
            html: html_content.to_string(),
            pictures_to_export: vec![],
        }
    }

    #[tokio::test]
    async fn test_export_page_with_pictures() {
        let temp_dir = tempdir().unwrap();
        let export_dir = temp_dir.path().join("export");
        let page_name = "test_with_pics".to_string();
        let exporter = ExporterImpl::new();

        let (page, expected_contents) =
            create_test_page_with_files(&temp_dir, "<html><body><h1>Hello</h1></body></html>", 2)
                .await;
        exporter
            .export_page(page.clone(), &page_name, &export_dir)
            .await
            .unwrap();

        // Verify HTML file
        let html_path = export_dir.join(make_html_file_name(&page_name));
        assert!(html_path.exists());
        let html_content = fs::read_to_string(html_path).await.unwrap();
        assert_eq!(html_content, page.html);

        // Verify resources directory and copied picture files
        let resources_path = export_dir.join(make_resource_dir_name(&page_name));
        assert!(resources_path.exists());
        assert!(resources_path.is_dir());

        for (i, pic_export) in page.pictures_to_export.iter().enumerate() {
            let pic_path = resources_path.join(&pic_export.target_file_name);
            assert!(pic_path.exists());
            let pic_content = fs::read_to_string(pic_path).await.unwrap();
            assert_eq!(pic_content, expected_contents[i]);
        }
    }

    #[tokio::test]
    async fn test_export_page_no_pictures() {
        let temp_dir = tempdir().unwrap();
        let export_dir = temp_dir.path().to_path_buf();
        let page_name = "test_no_pics".to_string();
        let exporter = ExporterImpl::new();

        let page = create_test_page_no_files("<html><body><h1>No Pics</h1></body></html>");
        exporter
            .export_page(page.clone(), &page_name, &export_dir)
            .await
            .unwrap();

        // Verify HTML file
        let html_name = make_html_file_name(&page_name);
        let html_path = export_dir.join(html_name);
        assert!(html_path.exists());
        let html_content = fs::read_to_string(html_path).await.unwrap();
        assert_eq!(html_content, page.html);

        // Verify resources directory is NOT created
        let resources_path = export_dir.join(make_resource_dir_name(&page_name));
        assert!(!resources_path.exists());
    }

    #[tokio::test]
    async fn test_export_overwrites_existing_html_file() {
        let temp_dir = tempdir().unwrap();
        let export_dir = temp_dir.path();
        let page_name = "overwrite_test".to_string();
        let html_file_name = make_html_file_name(&page_name);
        let html_path = export_dir.join(&html_file_name);

        fs::write(&html_path, "initial content").await.unwrap();

        let exporter = ExporterImpl::new();

        let new_html_content = "<html><body>overwritten content</body></html>";
        let page = create_test_page_no_files(new_html_content);
        exporter
            .export_page(page.clone(), &page_name, export_dir)
            .await
            .unwrap();

        let final_content = fs::read_to_string(&html_path).await.unwrap();
        assert_eq!(final_content, new_html_content);
    }

    #[tokio::test]
    async fn test_export_to_path_that_is_a_file_fails() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("i_am_a_file_not_a_dir");
        fs::write(&file_path, "hello").await.unwrap();

        let exporter = ExporterImpl::new();
        let page = create_test_page_no_files("test");
        let result = exporter.export_page(page, "any_name", &file_path).await;

        assert!(result.is_err());

        if let Err(Error::Io(e)) = result {
            assert_eq!(e.kind(), ErrorKind::AlreadyExists);
        } else {
            panic!("Expected Io error, but got {:?}", result);
        }
    }
}
