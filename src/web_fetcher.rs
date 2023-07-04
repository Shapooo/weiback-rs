use std::collections::HashMap;

use bytes::Bytes;
use futures::future::join_all;
use log::{debug, trace};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, IntoUrl, Response,
};
use serde_json::{from_str, Value};

use crate::data::{FavTag, LongText, Post, Posts};
use crate::error::{Error, Result};
use crate::utils::value_as_str;

const STATUSES_CONFIG_API: &str = "https://weibo.com/ajax/statuses/config";
const STATUSES_LONGTEXT_API: &str = "https://weibo.com/ajax/statuses/longtext";
const FAVORITES_ALL_FAV_API: &str = "https://weibo.com/ajax/favorites/all_fav";
const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";
const MOBILE_POST_API: &str = "https://m.weibo.cn/status";

#[derive(Debug)]
pub struct WebFetcher {
    web_client: Client,
    pic_client: Client,
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

    async fn _fetch(&self, url: impl IntoUrl, client: &Client) -> Result<Response> {
        Ok(client.get(url).send().await?)
    }

    pub async fn fetch_posts_meta(&self, uid: &str, page: u32) -> Result<Posts> {
        let url = format!("{FAVORITES_ALL_FAV_API}?uid={uid}&page={page}");
        debug!("fetch meta page, url: {url}");
        let res = self._fetch(url, &self.web_client).await?;
        let mut posts = res.json::<Value>().await?;
        trace!("get json: {posts:?}");
        if posts["ok"] != 1 {
            Err(Error::ResourceGetFailed("fetched data is not ok"))
        } else {
            if let Value::Array(v) = posts["data"].take() {
                let v: Result<Vec<_>> = join_all(v.into_iter().map(|p| self.preprocess_post(p)))
                    .await
                    .into_iter()
                    .collect();
                Ok(Posts { data: v? })
            } else {
                panic!("it should be a array, or weibo API has changed!")
            }
        }
    }

    async fn preprocess_post(&self, post: Post) -> Result<Post> {
        let mut post = self.preprocess_post_non_rec(post).await?;
        if post["retweeted_status"].is_object() {
            let retweet = self
                .preprocess_post_non_rec(post["retweeted_status"].take())
                .await?;
            post["retweeted_status"] = retweet;
        }
        Ok(post)
    }

    async fn preprocess_post_non_rec(&self, mut post: Post) -> Result<Post> {
        if !post["user"]["id"].is_number()
            && value_as_str(&post["text_raw"])?.starts_with("该内容请至手机客户端查看")
            && self.mobile_client.is_some()
        {
            post = self
                .fetch_mobile_page(value_as_str(&post["mblogid"])?)
                .await?;
        } else {
            if post["isLongText"] == true {
                let mblogid = value_as_str(&post["mblogid"])?;
                let long_text = self.fetch_long_text_content(mblogid).await?;
                post["text_raw"] = Value::String(long_text);
            }
        }
        Ok(post)
    }

    pub async fn fetch_pic(&self, url: impl IntoUrl) -> Result<Bytes> {
        debug!("fetch pic, url: {}", url.as_str());
        let res = self._fetch(url, &self.pic_client).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        Ok(res_bytes)
    }

    pub async fn fetch_emoticon(&self) -> Result<HashMap<String, String>> {
        let url = STATUSES_CONFIG_API;
        debug!("fetch emoticon, url: {url}");
        let res = self._fetch(url, &self.web_client).await?;
        let mut json: Value = res.json().await?;
        if json["ok"] != 1 {
            return Err(Error::ResourceGetFailed("fetched emoticon is not ok"));
        }

        let mut res = HashMap::new();
        let err = Err(Error::MalFormat(
            "the format of emoticon is unexpected".into(),
        ));
        let Value::Object(emoticon) = json["data"]["emoticon"].take() else {
            return err;
        };
        for (_, groups) in emoticon {
            let Value::Object(group) = groups else {
                return err;
            };
            for (_, emojis) in group {
                let Value::Array(emojis) = emojis else {
                    return err;
                };
                for mut emoji in emojis {
                    let (Value::String(phrase), Value::String(url)) =
                        (emoji["phrase"].take(), emoji["url"].take()) else {
                            return err;
                        };
                    res.insert(phrase, url);
                }
            }
        }
        Ok(res)
    }

    pub async fn fetch_mobile_page(&self, mblogid: &str) -> Result<Value> {
        if let Some(mobile_client) = &self.mobile_client {
            let url = format!("{}/{}", MOBILE_POST_API, mblogid);
            debug!("fetch mobile page, url: {}", &url);
            let res = self._fetch(url, mobile_client).await?;
            let text = res.text().await?;
            let Some(start) = text.find("\"status\":") else {
                return Err(Error::MalFormat(format!("malformed mobile post: {text}")));
            };
            let Some(end) = text.find("\"call\"") else {
                return Err(Error::MalFormat(format!("malformed mobile post: {text}")));
            };
            let Some( end) = *&text[..end].rfind(",") else {
                return Err(Error::MalFormat(format!("malformed mobile post: {text}")));
            } ;
            let mut post = from_str::<Value>(&text[start + 9..end])?;
            let id = value_as_str(&post["id"])?;
            let id = match id.parse::<i64>() {
                Ok(id) => id,
                Err(e) => {
                    return Err(Error::MalFormat(format!(
                        "failed to parse mobile post id {id}: {e}"
                    )))
                }
            };
            post["id"] = Value::Number(serde_json::Number::from(id));
            post["mblogid"] = Value::String(mblogid.to_owned());
            post["text_raw"] = post["text"].to_owned();
            if post["pics"].is_array() {
                if let Value::Array(pics) = post["pics"].take() {
                    post["pic_ids"] = serde_json::to_value(
                        pics.iter()
                            .map(|pic| Ok(value_as_str(&pic["pid"])?))
                            .collect::<Result<Vec<_>>>()?,
                    )
                    .unwrap();
                    post["pic_infos"] = serde_json::to_value(
                        pics.into_iter()
                            .map(|mut pic| {
                                let id = value_as_str(&pic["pid"])?.to_owned();
                                let mut v: HashMap<String, Value> = HashMap::new();
                                v.insert("pic_id".into(), pic["pid"].take());
                                v.insert("type".into(), "pic".into());
                                v.insert("large".into(), pic["large"].take());
                                v.insert(
                                    "bmiddle".into(),
                                    serde_json::json!({"url":pic["url"].take()}),
                                );
                                Ok((id, serde_json::to_value(v).unwrap()))
                            })
                            .collect::<Result<HashMap<String, Value>>>()?,
                    )
                    .unwrap();
                }
            }
            if post["retweeted_status"].is_object() {
                let id = value_as_str(&post["retweeted_status"]["id"])?;
                let id = match id.parse::<i64>() {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(Error::MalFormat(format!(
                            "failed to parse retweet id {id}: {e}"
                        )))
                    }
                };
                post["retweeted_status"]["id"] = Value::Number(serde_json::Number::from(id));
                post["retweeted_status"]["text_raw"] = post["retweeted_status"]["text"].to_owned();
            }

            Ok(post)
        } else {
            Err(Error::UnexpectedError("mobile cookie have not set"))
        }
    }

    pub async fn fetch_fav_total_num(&self) -> Result<u64> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let res = self._fetch(FAVORITES_TAGS_API, &self.web_client).await?;
        let fav_tag = res.json::<FavTag>().await?;
        trace!("get fav tag data: {:?}", fav_tag);
        assert_eq!(fav_tag.ok, 1);
        return Ok(fav_tag.fav_total_num);
    }

    pub async fn fetch_long_text_content(&self, mblogid: &str) -> Result<String> {
        let url = format!("{STATUSES_LONGTEXT_API}?id={mblogid}");
        debug!("fetch long text, url: {url}");
        let res = self._fetch(url, &self.web_client).await?;
        let long_text_meta = res.json::<LongText>().await?;
        long_text_meta.get_content()
    }
}
