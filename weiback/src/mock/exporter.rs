//! This module provides a mock implementation of the [`Exporter`] trait.
//!
//! `MockExporter` is designed for testing purposes, allowing verification of
//! export logic without actually writing files to the disk. It collects exported
//! `HTMLPage` objects in memory for inspection.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    error::{Error, Result},
    exporter::{Exporter, HTMLPage},
};

/// A mock implementation of the [`Exporter`] trait that captures exported pages in memory.
#[derive(Debug, Clone, Default)]
pub struct MockExporter {
    inner: Arc<Mutex<Inner>>,
}

/// Internal state for `MockExporter`.
#[derive(Debug, Default)]
struct Inner {
    /// List of HTML pages that have been "exported".
    exported_pages: Vec<HTMLPage>,
    /// Flag to simulate an export failure.
    should_fail: bool,
}

impl MockExporter {
    /// Creates a new `MockExporter` instance.
    pub fn new() -> Self {
        Default::default()
    }

    /// Retrieves a clone of all pages that have been "exported" by this mock.
    pub fn get_exported_pages(&self) -> Vec<HTMLPage> {
        self.inner.lock().unwrap().exported_pages.clone()
    }

    /// Sets whether the mock exporter should simulate a failure on subsequent `export_page` calls.
    ///
    /// # Arguments
    /// * `fail` - If `true`, `export_page` will return an error.
    pub fn set_should_fail(&self, fail: bool) {
        self.inner.lock().unwrap().should_fail = fail;
    }
}

#[async_trait]
impl Exporter for MockExporter {
    /// Simulates the export of an HTML page.
    ///
    /// If `should_fail` is set to `true`, this method will return an error. Otherwise,
    /// the page is added to the `exported_pages` list.
    ///
    /// # Arguments
    /// * `page` - The `HTMLPage` to export.
    /// * `_page_name` - The suggested filename for the page (ignored by mock).
    /// * `_export_dir` - The target directory for export (ignored by mock).
    ///
    /// # Errors
    /// Returns `Error::InconsistentTask` if `should_fail` is true.
    async fn export_page(
        &self,
        page: HTMLPage,
        _page_name: &str,
        _export_dir: &Path,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.should_fail {
            return Err(Error::InconsistentTask("Mock error".into()));
        }
        inner.exported_pages.push(page);
        Ok(())
    }
}
