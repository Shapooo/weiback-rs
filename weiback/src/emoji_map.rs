use std::collections::HashMap;

use log::warn;
use tokio::sync::OnceCell;
use weibosdk_rs::EmojiUpdateAPI;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct EmojiMap<W: EmojiUpdateAPI> {
    api_client: W,
    emoji_map: OnceCell<HashMap<String, String>>,
}

impl<W: EmojiUpdateAPI> EmojiMap<W> {
    pub fn new(api_client: W) -> Self {
        Self {
            api_client,
            emoji_map: Default::default(),
        }
    }

    pub async fn get_or_try_init(&self) -> Result<&HashMap<String, String>> {
        Ok(self
            .emoji_map
            .get_or_try_init(async || self.api_client.emoji_update().await)
            .await
            .map_err(|e| {
                warn!("{e}");
                e
            })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use std::path::Path;
    use weibosdk_rs::mock::{MockAPI, MockClient};

    #[tokio::test]
    async fn test_get_emoji_fail() {
        let client = MockClient::new();
        let api = MockAPI::new(client.clone());
        client.set_emoji_update_response_from_str("");
        let emoji_map = EmojiMap::new(api.clone());
        let res = emoji_map.get_or_try_init().await;
        assert!(matches!(res, Err(Error::FormatError(..))));
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        client
            .set_emoji_update_response_from_file(
                manifest_dir.join("tests/data/emoji.json").as_path(),
            )
            .unwrap();
        client
            .set_web_emoticon_response_from_file(
                manifest_dir.join("tests/data/web_emoji.json").as_path(),
            )
            .unwrap();
        let reference_emoji = api.emoji_update().await.unwrap();
        let emoji = emoji_map.get_or_try_init().await.unwrap();
        assert_eq!(&reference_emoji, emoji);
    }
}
