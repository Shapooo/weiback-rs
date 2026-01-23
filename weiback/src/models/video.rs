use bytes::Bytes;
use url::Url;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Video {
    pub meta: VideoMeta,
    pub blob: Bytes,
}

#[derive(Debug, Clone)]
pub struct VideoMeta {
    pub url: Url,
    pub post_id: i64,
}

impl VideoMeta {
    pub fn new(url: &str, post_id: i64) -> Result<Self> {
        Ok(VideoMeta {
            url: Url::parse(url)?,
            post_id,
        })
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}
