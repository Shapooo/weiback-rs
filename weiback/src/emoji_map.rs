//! This module provides the [`EmojiMap`], which manages the mapping of emoji text
//! (e.g., `[doge]`) to their corresponding image URLs.
//!
//! The mapping is lazily initialized using [`OnceCell`] to ensure that the API
//! request for the emoji list is only made when first needed.

use std::collections::HashMap;

use tokio::sync::OnceCell;
use tracing::warn;
use url::Url;

use crate::api::EmojiUpdateApi;
use crate::error::Result;

/// A thread-safe, lazily-initialized map for Weibo emojis.
#[derive(Debug, Clone)]
pub struct EmojiMap<E: EmojiUpdateApi> {
    api_client: E,
    emoji_map: OnceCell<HashMap<String, Url>>,
}

impl<W: EmojiUpdateApi> EmojiMap<W> {
    /// Creates a new `EmojiMap` instance.
    ///
    /// # Arguments
    /// * `api_client` - An implementor of [`EmojiUpdateApi`] used to fetch the emoji list.
    pub fn new(api_client: W) -> Self {
        Self {
            api_client,
            emoji_map: Default::default(),
        }
    }

    /// Returns the emoji map, initializing it if it hasn't been yet.
    ///
    /// This method is idempotent and thread-safe. Subsequent calls will return
    /// the cached mapping.
    ///
    /// # Returns
    /// A `Result` containing a reference to the `HashMap<String, Url>`.
    ///
    /// # Errors
    /// Returns an error if the underlying API call to fetch emojis fails.
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
mod local_tests {
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
