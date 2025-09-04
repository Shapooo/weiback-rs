#![allow(async_fn_in_trait)]
use std::collections::HashMap;

use log::{debug, error, info};
use serde::Deserialize;
use serde_json::Value;
use weibosdk_rs::http_client::{HttpClient, HttpResponse};

use super::ApiClientImpl;
use crate::error::{Error, Result};
use crate::models::err_response::ErrResponse;

#[derive(Debug, Clone, Deserialize)]
struct Emoji {
    key: String,
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct EmojiData {
    card: Vec<Emoji>,
}

#[derive(Debug, Clone, Deserialize)]
struct EmojiUpdateResponseInner {
    data: EmojiData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum EmojiUpdateResponse {
    Succ(EmojiUpdateResponseInner),
    Fail(ErrResponse),
}

pub trait EmojiUpdateApi {
    async fn emoji_update(&self) -> Result<HashMap<String, String>>;
}

impl<C: HttpClient> EmojiUpdateApi for ApiClientImpl<C> {
    async fn emoji_update(&self) -> Result<HashMap<String, String>> {
        info!("getting emoji update");
        let mut res = self.fetch_from_mobile_api().await?;
        res.extend(self.fetch_from_web_api().await?);
        debug!("emoji update success, got {} emojis", res.len());
        Ok(res)
    }
}

impl<C: HttpClient> ApiClientImpl<C> {
    async fn fetch_from_web_api(&self) -> Result<HashMap<String, String>> {
        debug!("fetch emoticon");
        let res = self.client.fetch_from_web_api().await?;
        let mut json: Value = res.json().await?;
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
                    res.insert(phrase, url);
                }
            }
        }
        Ok(res)
    }

    async fn fetch_from_mobile_api(&self) -> Result<HashMap<String, String>> {
        let response = self.client.fetch_from_mobile_api().await?;
        let res = response.json::<EmojiUpdateResponse>().await?;
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
    use std::sync::{Arc, Mutex};

    use weibosdk_rs::{ApiClient as SdkApiClient, mock::MockClient, session::Session};

    #[tokio::test]
    async fn test_emoji_update() {
        let mock_client = MockClient::new();
        let session = Session {
            gsid: "test_gsid".to_string(),
            uid: "test_uid".to_string(),
            screen_name: "test_screen_name".to_string(),
            cookie_store: Default::default(),
        };
        let session = Arc::new(Mutex::new(session));
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
            emoji_map.get("[光夜萧逸]").unwrap(),
            "https://d.sinaimg.cn/prd/100/1378/2025/06/24/2025_LoveOsborn_mobile.png"
        );
    }
}

#[cfg(test)]
mod real_tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use weibosdk_rs::{ApiClient as SdkApiClient, http_client, session::Session};

    use super::*;

    #[tokio::test]
    async fn test_real_emoji_update() {
        let session_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("session.json");
        let session = Arc::new(Mutex::new(Session::load(session_file).unwrap()));
        let client = http_client::Client::new().unwrap();
        let mut api_client = SdkApiClient::new(client, Default::default());
        api_client.login_with_session(session).await.unwrap();
        let api_client = ApiClientImpl::new(api_client);
        let emoji_map = api_client.emoji_update().await.unwrap();
        assert!(!emoji_map.is_empty());
    }
}
