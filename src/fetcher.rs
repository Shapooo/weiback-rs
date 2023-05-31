#![allow(dead_code)]
#![allow(unused)]
use std::mem::swap;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use log::{debug, error, info, trace};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, IntoUrl, Response,
};
use serde_json::{value::Number, Value};

use crate::meta_data::{FavTag, Posts, Post};

const STATUSES_CONFIG_API: &str = "https://weibo.com/ajax/statuses/config";
const STATUSES_MY_MICRO_BLOG_API: &str = "https://weibo.com/ajax/statuses/mymblog";
const STATUSES_LONGTEXT_API: &str = "https://weibo.com/ajax/statuses/longtext";
const STATUSES_LIKE_LIST_API: &str = "https://weibo.com/ajax/statuses/likelist";
const FAVORITES_ALL_FAV_API: &str = "https://weibo.com/ajax/favorites/all_fav";
const FAVORITES_TAGS_API: &str = "https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1";
const PROFILE_INFO_API: &str = "https://weibo.com/ajax/profile/info";

#[derive(Debug)]
pub struct Fetcher {
    post_client: Client,
    pic_client: Client,
    mobile_client: Client,
}

impl Fetcher {
    pub fn new(web_cookie: String, mobile_cookie: Option<String>) -> Self {
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            "Cookie",
            HeaderValue::from_str(web_cookie.as_str()).unwrap(),
        );
        headers.insert("Host", HeaderValue::from_static("weibo.com"));
        headers.insert("Referer", HeaderValue::from_static("https://weibo.com/"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/113.0",
            ),
        );
        headers.insert(
            "Accept-Language",
            HeaderValue::from_static("en-US,en;q=0.5"),
        );
        headers.insert(
            "Accept-Encoding",
            HeaderValue::from_static("gzip, deflate, br"),
        );
        headers.insert(
            "X-Requested-With",
            HeaderValue::from_static("XMLHttpRequest"),
        );
        headers.insert("client-version", HeaderValue::from_static("v2.40.55"));
        headers.insert("server-version", HeaderValue::from_static("v2023.05.23.3"));
        headers.insert("DNT", HeaderValue::from_static("1"));
        headers.insert("Connection", HeaderValue::from_static("keep-alive"));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Pragma", HeaderValue::from_static("no-cache"));
        headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
        headers.insert("TE", HeaderValue::from_static("trailers"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        return Fetcher {
            post_client: client.clone(),
            pic_client: client.clone(),
            mobile_client: client.clone(),
        };
    }

    async fn fetch(&self, url: impl IntoUrl, client: &Client) -> Result<Response> {
        Ok(client.get(url).send().await?)
    }

    pub async fn fetch_posts_meta(&self, uid: &str, page: u64) -> Result<Vec<Post>> {
        let url = format!("{FAVORITES_ALL_FAV_API}?uid={uid}&page={page}");
        debug!("fetch meta page, url: {url}");
        let res = self.fetch(url, &self.post_client).await?;
        let mut posts = res.json::<Posts>().await?;
        trace!("get json: {posts:?}");
        if posts.ok != 1 {
            Err(anyhow!("fetched data is not ok"))
        } else {
            Ok(posts.data)
        }
    }

    pub async fn fetch_pic(&self, url: impl IntoUrl) -> Result<Bytes> {
        debug!("fetch pic, url: {}", url.as_str());
        let res = self.fetch(url, &self.pic_client).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        Ok(res_bytes)
    }

    pub async fn fetch_mobile_page(&self, url: impl IntoUrl) -> Result<Value> {
        unimplemented!()
    }

    pub async fn get_fav_total_num(&self) -> Result<u64> {
        debug!("fetch fav page sum, url: {}", FAVORITES_TAGS_API);
        let res = self.fetch(FAVORITES_TAGS_API, &self.post_client).await?;
        let fav_tag = res.json::<FavTag>().await?;
        trace!("get fav tag data: {:?}", fav_tag);
        assert_eq!(fav_tag.ok, 1);
        return Ok(fav_tag.fav_total_num);
    }

    pub async fn get_long_text(&self) -> Result<String> {
        unimplemented!()
    }
}
