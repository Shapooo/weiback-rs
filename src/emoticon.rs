use crate::{
    error::{Error, Result},
    web_fetcher::WebFetcher,
};

use std::collections::HashMap;

use log::debug;
use serde_json::Value;

static mut EMOTICON: Option<HashMap<String, String>> = None;
const STATUSES_CONFIG_API: &str = "https://weibo.com/ajax/statuses/config";

pub async fn init_emoticon(fetcher: &WebFetcher) -> Result<()> {
    let url = STATUSES_CONFIG_API;
    debug!("fetch emoticon, url: {url}");
    let res = fetcher.get(url, fetcher.web_client()).await?;
    let mut json: Value = res.json().await?;
    if json["ok"] != 1 {
        return Err(Error::ResourceGetFailed(format!(
            "fetched emoticon is not ok: {json:?}"
        )));
    }

    let mut res = HashMap::new();
    let Value::Object(emoticon) = json["data"]["emoticon"].take() else {
        return Err(Error::MalFormat(
            "the format of emoticon is unexpected".into(),
        ));
    };
    for (_, groups) in emoticon {
        let Value::Object(group) = groups else {
            return Err(Error::MalFormat(
                "the format of emoticon is unexpected".into(),
            ));
        };
        for (_, emojis) in group {
            let Value::Array(emojis) = emojis else {
                return Err(Error::MalFormat(
                    "the format of emoticon is unexpected".into(),
                ));
            };
            for mut emoji in emojis {
                let (Value::String(phrase), Value::String(url)) =
                    (emoji["phrase"].take(), emoji["url"].take())
                else {
                    return Err(Error::MalFormat(
                        "the format of emoticon is unexpected".into(),
                    ));
                };
                res.insert(phrase, url);
            }
        }
    }
    unsafe {
        EMOTICON = Some(res);
    }

    Ok(())
}

pub fn emoticon_get(key: &str) -> Option<&str> {
    unsafe {
        EMOTICON
            .as_ref()
            .and_then(|map| map.get(key).map(|v| v.as_str()))
    }
}
