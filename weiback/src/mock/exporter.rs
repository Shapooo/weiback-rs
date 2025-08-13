//! Test mock for exporter
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    error::{Error, Result},
    exporter::{Exporter, HTMLPage},
};

#[derive(Debug, Clone, Default)]
pub struct ExporterMock {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    exported_pages: Vec<HTMLPage>,
    should_fail: bool,
}

impl ExporterMock {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_exported_pages(&self) -> Vec<HTMLPage> {
        self.inner.lock().unwrap().exported_pages.clone()
    }

    pub fn set_should_fail(&self, fail: bool) {
        self.inner.lock().unwrap().should_fail = fail;
    }
}

impl Exporter for ExporterMock {
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
