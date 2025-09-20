use std::collections::HashMap;

use log::warn;
use tokio::sync::OnceCell;
use url::Url;

use crate::api::EmojiUpdateApi;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct EmojiMap<E: EmojiUpdateApi> {
    api_client: E,
    emoji_map: OnceCell<HashMap<String, Url>>,
}

impl<W: EmojiUpdateApi> EmojiMap<W> {
    pub fn new(api_client: W) -> Self {
        Self {
            api_client,
            emoji_map: Default::default(),
        }
    }

    pub async fn get_or_try_init(&self) -> Result<&HashMap<String, Url>> {
        self.emoji_map
            .get_or_try_init(async || self.api_client.emoji_update().await)
            .await
            .map_err(|e| {
                warn!("{e}");
                e
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use weibosdk_rs::mock::MockClient;

    use crate::error::Error;
    use crate::mock::MockApi;

    #[tokio::test]
    async fn test_get_emoji_fail() {
        let client = MockClient::new();
        let api = MockApi::new(client.clone());
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
