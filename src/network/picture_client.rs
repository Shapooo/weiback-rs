#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PictureType {
    InPost { url: String, post_id: i64 },
    Avatar { url: String, user_id: i64 },
    Emoji { url: String },
    Temporary { url: String },
}

#[derive(Debug, Clone)]
pub struct PictureInternal {
    pub type_: PictureType,
    pub blob: Option<PictureBlob>,
}

impl PictureInternal {
    async fn fetch_blob(&self, fetcher: &NetworkImpl) -> Result<Bytes> {
        let url = self.get_url();
        debug!("fetch pic, url: {}", url);
        let res = fetcher.get(url).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        Ok(res_bytes)
    }
}
