use anyhow::Result;
use log::{debug, trace};

use super::HttpClient;
use crate::models::Picture;

pub struct PictureClient {
    http_client: HttpClient,
}

impl PictureClient {
    pub fn new(http_client: HttpClient) -> Self {
        Self { http_client }
    }

    pub async fn get_blob(&self, pic: &mut Picture) -> Result<()> {
        let url = pic.get_url();
        debug!("fetch pic, url: {}", url);
        let res = self.http_client.get(url).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        pic.set_blob(res_bytes);
        Ok(())
    }
}
