use std::path::Path;

use anyhow;

use crate::generator::HTMLPage;

#[derive(Debug, Clone)]
pub struct Exporter();

impl Exporter {
    pub fn new() -> Self {
        Exporter()
    }

    pub async fn export_page<P>(&self, page: HTMLPage, path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        todo!()
    }
}
