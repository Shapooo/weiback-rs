use anyhow::Result;
use log::{debug, trace};

use super::NetworkImpl;
use crate::app::models::Picture;

struct PictureClient<'a> {
    network: &'a NetworkImpl,
}

impl<'a> PictureClient<'a> {
    async fn fetch_blob(&self, pic: &mut Picture) -> Result<()> {
        let url = pic.get_url();
        debug!("fetch pic, url: {}", url);
        let res = self.network.get(url).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        pic.set_blob(res_bytes);
        Ok(())
    }
}
