//! This module provides functionality for fetching and updating emoji (emoticon) data
//! from the Weibo API.
//!
//! It includes logic to retrieve emoji from both the mobile and web versions of the API,
//! consolidate them, and handle various response formats and errors.
use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, error, info};
use url::Url;
use weibosdk_rs::http_client::{HttpClient, HttpResponse};

use super::ApiClientImpl;
use crate::error::{Error, Result};
use crate::models::err_response::ErrResponse;

/// Represents a single emoji with its key (phrase) and URL.
#[derive(Debug, Clone, Deserialize)]
struct Emoji {
    key: String,
    url: Url,
}

/// Represents the structure containing a list of emojis in the mobile API response.
#[derive(Debug, Clone, Deserialize)]
struct EmojiData {
    card: Vec<Emoji>,
}

/// Represents the successful response structure from the mobile emoji update API.
#[derive(Debug, Clone, Deserialize)]
struct EmojiUpdateResponseInner {
    data: EmojiData,
}

/// An enum representing the possible responses from the mobile emoji update API,
/// which can either be a successful data payload or an error.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum EmojiUpdateResponse {
    Succ(EmojiUpdateResponseInner),
    Fail(ErrResponse),
}

/// Trait for API clients that can fetch and update emoji data.
#[async_trait]
pub trait EmojiUpdateApi {
    /// Fetches and consolidates emoji data from both mobile and web Weibo APIs.
    ///
    /// # Returns
    /// A `Result` containing a `HashMap` where keys are emoji phrases (e.g., `[哈哈]`)
    /// and values are their corresponding image URLs.
    async fn emoji_update(&self) -> Result<HashMap<String, Url>>;
}

#[async_trait]
impl<C: HttpClient> EmojiUpdateApi for ApiClientImpl<C> {
    async fn emoji_update(&self) -> Result<HashMap<String, Url>> {
        info!("getting emoji update");
        let mut res = self.fetch_from_mobile_api().await?;
        res.extend(self.fetch_from_web_api().await?);
        debug!("emoji update success, got {} emojis", res.len());
        Ok(res)
    }
}

impl<C: HttpClient> ApiClientImpl<C> {
    /// Fetches emoji data from the web version of the Weibo API.
    ///
    /// This method parses a complex JSON structure specific to the web API's emoticon data.
    ///
    /// # Returns
    /// A `Result` containing a `HashMap` of emoji phrases to URLs on success, or an `Error` on failure.
    async fn fetch_from_web_api(&self) -> Result<HashMap<String, Url>> {
        debug!("fetch emoticon");
        let res = self.client.fetch_from_web_api().await.inspect_err(|e| {
            error!("fetch_from_web_api failed: {e}");
        })?;
        let mut json: Value = res.json().await.inspect_err(|e| {
            error!("parse web emoticon response failed: {e}");
        })?;
        if json["ok"] != 1 {
            let err_res = ErrResponse {
                errmsg: json["url"].as_str().unwrap_or_default().to_string(),
                errno: json["ok"].as_i64().unwrap_or(-100) as i32,
                errtype: Default::default(),
                isblock: Default::default(),
            };
            return Err(Error::ApiError(err_res));
        }

        let mut res = HashMap::new();
        let Value::Object(emoticon) = json["data"]["emoticon"].take() else {
            return Err(Error::FormatError(
                "the format of emoticon is unexpected".to_string(),
            ));
        };
        for (_, groups) in emoticon {
            let Value::Object(group) = groups else {
                return Err(Error::FormatError(
                    "the format of emoticon is unexpected".to_string(),
                ));
            };
            for (_, emojis) in group {
                let Value::Array(emojis) = emojis else {
                    return Err(Error::FormatError(
                        "the format of emoticon is unexpected".to_string(),
                    ));
                };
                for mut emoji in emojis {
                    let (Value::String(phrase), Value::String(url)) =
                        (emoji["phrase"].take(), emoji["url"].take())
                    else {
                        return Err(Error::FormatError(
                            "the format of emoticon is unexpected".to_string(),
                        ));
                    };
                    let url = Url::parse(&url).inspect_err(|e| {
                        error!("parse emoji url '{}' failed: {e}", url);
                    })?;
                    res.insert(phrase, url);
                }
            }
        }
        Ok(res)
    }

    /// Fetches emoji data from the mobile version of the Weibo API.
    ///
    /// This method expects a structured JSON response that is deserialized into `EmojiUpdateResponse`.
    ///
    /// # Returns
    /// A `Result` containing a `HashMap` of emoji phrases to URLs on success, or an `Error` on failure.
    async fn fetch_from_mobile_api(&self) -> Result<HashMap<String, Url>> {
        let response = self.client.fetch_from_mobile_api().await.inspect_err(|e| {
            error!("fetch_from_mobile_api failed: {e}");
        })?;
        let res = response
            .json::<EmojiUpdateResponse>()
            .await
            .inspect_err(|e| {
                error!("parse EmojiUpdateResponse failed: {e}");
            })?;
        match res {
            EmojiUpdateResponse::Succ(data) => {
                let mut emoji_map = HashMap::new();
                for emoji in data.data.card {
                    emoji_map.insert(emoji.key, emoji.url);
                }
                Ok(emoji_map)
            }
            EmojiUpdateResponse::Fail(err) => {
                error!("emoji update failed: {err:?}");
                Err(Error::ApiError(err))
            }
        }
    }
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use std::path::Path;

    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_emoji_update() {
        let mock_client = MockClient::new();
        let session = Session {
            gsid: "test_gsid".to_string(),
            uid: "test_uid".to_string(),
            user: Value::Null,
            cookie_store: Default::default(),
        };
        let api_client =
            ApiClientImpl::new(SdkApiClient::from_session(mock_client.clone(), session));

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        mock_client
            .set_web_emoticon_response_from_file(&manifest_dir.join("tests/data/web_emoji.json"))
            .unwrap();
        mock_client
            .set_emoji_update_response_from_file(&manifest_dir.join("tests/data/emoji.json"))
            .unwrap();

        let emoji_map = api_client.emoji_update().await.unwrap();
        assert!(!emoji_map.is_empty());
        assert!(emoji_map.contains_key("[光夜萧逸]"));
        assert_eq!(
            emoji_map.get("[光夜萧逸]").unwrap().as_str(),
            "https://d.sinaimg.cn/prd/100/1378/2025/06/24/2025_LoveOsborn_mobile.png"
        );
    }
}

#[cfg(test)]
mod real_tests {
    use std::path::Path;

    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    use super::*;

    #[tokio::test]
    async fn test_real_emoji_update() {
        let session_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("session.json");
        let session = Session::load(session_file).unwrap();
        let client = http_client::Client::new().unwrap();
        let api_client = SdkApiClient::new(client, Default::default());
        api_client.login_with_session(session).await.unwrap();
        let api_client = ApiClientImpl::new(api_client);
        let emoji_map = api_client.emoji_update().await.unwrap();
        assert!(!emoji_map.is_empty());
    }
}
