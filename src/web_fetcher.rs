use std::collections::HashMap;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures::future::join_all;
use log::{debug, trace};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, IntoUrl, Response,
};
use serde_json::{from_str, Value};

use crate::data::{FavTag, LongText, Post, Posts};

const STATUSES_CONFIG_API: &str = "https://weibo.com/ajax/statuses/config";
#[allow(unused)]
const STATUSES_MY_MICRO_BLOG_API: &str = "https://weibo.com/ajax/statuses/mymblog";
const STATUSES_LONGTEXT_API: &str = "https://weibo.com/ajax/statuses/longtext";
#[allow(unused)]
const STATUSES_LIKE_LIST_API: &str = "https://weibo.com/ajax/statuses/likelist";
const FAVORITES_ALL_FAV_API: &str = "https://weibo.com/ajax/favorites/all_fav";
#[allow(unused)]
const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";
#[allow(unused)]
const PROFILE_INFO_API: &str = "https://weibo.com/ajax/profile/info";
const MOBILE_POST_API: &str = "https://m.weibo.cn/status";

#[derive(Debug)]
pub struct WebFetcher {
    web_client: Client,
    pic_client: Client,
    #[allow(unused)]
    mobile_client: Option<Client>,
}

impl WebFetcher {
    pub fn new(web_cookie: String, mobile_cookie: Option<String>) -> Self {
        let mut web_headers: HeaderMap = HeaderMap::new();
        web_headers.insert(
            "Accept",
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        web_headers.insert(
            "Cookie",
            HeaderValue::from_str(web_cookie.as_str()).unwrap(),
        );
        web_headers.insert("Host", HeaderValue::from_static("weibo.com"));
        web_headers.insert("Referer", HeaderValue::from_static("https://weibo.com/"));
        web_headers.insert(
            "User-Agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/113.0",
            ),
        );
        web_headers.insert(
            "Accept-Language",
            HeaderValue::from_static("en-US,en;q=0.5"),
        );
        web_headers.insert(
            "Accept-Encoding",
            HeaderValue::from_static("gzip, deflate, br"),
        );
        web_headers.insert(
            "X-Requested-With",
            HeaderValue::from_static("XMLHttpRequest"),
        );
        web_headers.insert("client-version", HeaderValue::from_static("v2.40.55"));
        web_headers.insert("server-version", HeaderValue::from_static("v2023.05.23.3"));
        web_headers.insert("DNT", HeaderValue::from_static("1"));
        web_headers.insert("Connection", HeaderValue::from_static("keep-alive"));
        web_headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        web_headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        web_headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        web_headers.insert("Pragma", HeaderValue::from_static("no-cache"));
        web_headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
        web_headers.insert("TE", HeaderValue::from_static("trailers"));

        let web_client = reqwest::Client::builder()
            .default_headers(web_headers)
            .build()
            .unwrap();

        let mut pic_headers = HeaderMap::new();
        pic_headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/113.0",
            ),
        );
        pic_headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("image/avif,image/webp,*/*"),
        );
        pic_headers.insert(
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.5"),
        );
        pic_headers.insert(
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br"),
        );
        pic_headers.insert(header::DNT, HeaderValue::from_static("1"));
        pic_headers.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
        pic_headers.insert(
            header::REFERER,
            HeaderValue::from_static("https://weibo.com/"),
        );
        pic_headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
        pic_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        pic_headers.insert(header::TE, HeaderValue::from_static("trailers"));
        pic_headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("image"));
        pic_headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("no-cors"));
        pic_headers.insert("Sec-Fetch-Site", HeaderValue::from_static("cross-site"));
        let pic_client = reqwest::Client::builder()
            .default_headers(pic_headers)
            .build()
            .unwrap();

        let mobile_client = mobile_cookie.map(|cookie| {
            let mut mobile_headers = HeaderMap::new();
            mobile_headers.insert("Cookie", HeaderValue::from_str(&cookie).unwrap());
            mobile_headers.insert(
                "Accept",
                HeaderValue::from_static("application/json, text/plain, */*"),
            );
            mobile_headers.insert(
                "Accept-Language",
                HeaderValue::from_static("en-US,en;q=0.5"),
            );
            mobile_headers.insert(
                "Accept-Encoding",
                HeaderValue::from_static("gzip, deflate, br"),
            );
            mobile_headers.insert(
                "X-Requested-With",
                HeaderValue::from_static("XMLHttpRequest"),
            );
            mobile_headers.insert("client-version", HeaderValue::from_static("v2.40.57"));
            mobile_headers.insert("server-version", HeaderValue::from_static("v2023.05.30.1"));
            mobile_headers.insert("DNT", HeaderValue::from_static("1"));
            mobile_headers.insert("Connection", HeaderValue::from_static("keep-alive"));
            mobile_headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
            mobile_headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
            mobile_headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
            mobile_headers.insert("Pragma", HeaderValue::from_static("no-cache"));
            mobile_headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
            mobile_headers.insert("TE", HeaderValue::from_static("trailers"));

            let client = reqwest::Client::builder()
                .default_headers(mobile_headers)
                .build()
                .unwrap();

            client
        });

        return WebFetcher {
            web_client,
            pic_client,
            mobile_client,
        };
    }

    async fn fetch(&self, url: impl IntoUrl, client: &Client) -> Result<Response> {
        Ok(client.get(url).send().await?)
    }

    pub async fn fetch_posts_meta(&self, uid: &str, page: u32) -> Result<Posts> {
        let url = format!("{FAVORITES_ALL_FAV_API}?uid={uid}&page={page}");
        debug!("fetch meta page, url: {url}");
        let res = self.fetch(url, &self.web_client).await?;
        let mut posts = res.json::<Value>().await?;
        trace!("get json: {posts:?}");
        if posts["ok"].as_i64().unwrap() != 1 {
            Err(anyhow!("fetched data is not ok: {:?}", posts))
        } else {
            if let Value::Array(v) = posts["data"].take() {
                let v: Result<Vec<_>> = join_all(v.into_iter().map(|p| self.preprocess_post(p)))
                    .await
                    .into_iter()
                    .collect();
                Ok(Posts { data: v.unwrap() })
            } else {
                panic!("it should be a array, or weibo API has changed!")
            }
        }
    }

    async fn preprocess_post(&self, mut post: Post) -> Result<Post> {
        if !post["user"]["id"].is_number()
            && post["text_raw"]
                .as_str()
                .unwrap()
                .starts_with("该内容请至手机客户端查看")
        {
            post = self
                .fetch_mobile_page(post["mblogid"].as_str().unwrap())
                .await?;
        }
        let is_long_text = &post["isLongText"];
        if is_long_text.is_boolean() && is_long_text.as_bool().unwrap() {
            let mblogid = &post["mblogid"];
            let long_text = self
                .fetch_long_text_content(mblogid.as_str().unwrap())
                .await?;
            post["text_raw"] = Value::String(long_text);
        }
        Ok(post)
    }

    pub async fn fetch_pic(&self, url: impl IntoUrl) -> Result<Bytes> {
        debug!("fetch pic, url: {}", url.as_str());
        let res = self.fetch(url, &self.pic_client).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        Ok(res_bytes)
    }

    pub async fn fetch_emoticon(&self) -> Result<HashMap<String, String>> {
        let url = STATUSES_CONFIG_API;
        debug!("fetch emoticon, url: {url}");
        let res = self.fetch(url, &self.web_client).await?;
        let mut json: Value = res.json().await?;
        if json["ok"] != 1 {
            return Err(anyhow!("fetched emoticon is not ok"));
        }
        let mut res = HashMap::new();
        if let Value::Object(emoticon) = json["data"]["emoticon"].take() {
            for (_, groups) in emoticon {
                if let Value::Object(group) = groups {
                    for (_, emojis) in group {
                        if let Value::Array(emojis) = emojis {
                            for mut emoji in emojis {
                                if let (Value::String(phrase), Value::String(url)) =
                                    (emoji["phrase"].take(), emoji["url"].take())
                                {
                                    res.insert(phrase, url);
                                } else {
                                    return Err(anyhow!("the format of emoticon is unexpected"));
                                }
                            }
                        } else {
                            return Err(anyhow!("the format of emoticon is unexpected"));
                        }
                    }
                } else {
                    return Err(anyhow!("the format of emoticon is unexpected"));
                }
            }
        } else {
            return Err(anyhow!("the format of emoticon is unexpected"));
        }

        Ok(res)
    }

    #[allow(unused)]
    pub async fn fetch_mobile_page(&self, mblogid: &str) -> Result<Value> {
        if let Some(mobile_client) = &self.mobile_client {
            let url = format!("{}/{}", MOBILE_POST_API, mblogid);
            let res = self.fetch(url, mobile_client).await?;
            let text = res.text().await?;
            let start = text.find("\"status\":").unwrap();
            let end = text.find("\"call\"").unwrap();
            let end = *&text[..end].rfind(",").unwrap();
            Ok(from_str::<Value>(&text[start + 9..end])?)
        } else {
            Err(anyhow!("mobile cookie have not set"))
        }
    }

    pub async fn fetch_fav_total_num(&self) -> Result<u64> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let res = self.fetch(FAVORITES_TAGS_API, &self.web_client).await?;
        let fav_tag = res.json::<FavTag>().await?;
        trace!("get fav tag data: {:?}", fav_tag);
        assert_eq!(fav_tag.ok, 1);
        return Ok(fav_tag.fav_total_num);
    }

    pub async fn fetch_long_text_content(&self, mblogid: &str) -> Result<String> {
        let url = format!("{STATUSES_LONGTEXT_API}?id={mblogid}");
        debug!("fetch long text, url: {url}");
        let res = self.fetch(url, &self.web_client).await?;
        let long_text_meta = res.json::<LongText>().await?;
        Ok(long_text_meta.get_content()?)
    }
}

#[cfg(test)]
mod web_fetcher_test {
    use super::*;
    #[tokio::test]
    async fn fetch_emoticon() {
        let f = WebFetcher::new("[privacy]".into(), None);
        let res = f.fetch_emoticon().await.unwrap();
        println!("{:?}", res);
    }
}
