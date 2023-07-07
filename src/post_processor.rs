use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::path::Path;

use bytes::Bytes;
use futures::future::join_all;
use lazy_static::lazy_static;
use log::{debug, trace};
use regex::Regex;
use serde_json::{from_str, to_value, Value};

use crate::data::{Post, Posts};
use crate::error::{Error, Result};
use crate::exporter::{HTMLPage, Picture};
use crate::html_generator::HTMLGenerator;
use crate::persister::Persister;
use crate::utils::{pic_url_to_file, value_as_str};
use crate::web_fetcher::WebFetcher;

lazy_static! {
    static ref NEWLINE_EXPR: Regex = Regex::new("\\n").unwrap();
    static ref URL_EXPR: Regex =
        Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}").unwrap();
    static ref AT_EXPR: Regex = Regex::new(r#"@[\u4e00-\u9fa5|\uE7C7-\uE7F3|\w_\-·]+"#).unwrap();
    static ref EMOJI_EXPR: Regex = Regex::new(r#"(\[.*?\])"#).unwrap();
    static ref EMAIL_EXPR: Regex =
        Regex::new(r#"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}"#).unwrap();
    static ref TOPIC_EXPR: Regex = Regex::new(r#"#([^#]+)#"#).unwrap();
}

#[derive(Debug)]
pub struct PostProcessor {
    html_generator: HTMLGenerator,
    web_fetcher: WebFetcher,
    persister: Persister,
    emoticon: HashMap<String, String>,
}

impl PostProcessor {
    pub fn new(web_fetcher: WebFetcher, persister: Persister) -> Self {
        Self {
            html_generator: HTMLGenerator::new(),
            web_fetcher,
            persister,
            emoticon: Default::default(),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        debug!("initing...");
        self.emoticon = self.web_fetcher.fetch_emoticon().await?;
        self.persister.init().await?;
        Ok(())
    }

    pub async fn download_fav_posts(&self, uid: &str, page: u32, with_pic: bool) -> Result<usize> {
        let posts = self.web_fetcher.fetch_posts_meta(uid, page).await?;
        let posts = join_all(posts.into_iter().map(|post| async {
            let post = self.preprocess_post(post).await?;
            self.persister.insert_post(&post).await?;
            Ok(post)
        }))
        .await
        .into_iter()
        .collect::<Result<Posts>>()?;
        Ok(posts)
    }

    pub async fn get_fav_post_from_db(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Posts> {
        let limit = (range.end() - range.start()) + 1;
        let offset = *range.start() - 1;
        self.persister.query_posts(limit, offset, reverse).await
    }

    pub async fn save_post_pictures(&self, posts: Posts) -> Result<()> {
        debug!("save pictures of posts to db...");
        let mut pics = posts
            .iter()
            .map(|ref post| Ok(self.extract_emoji_from_text(value_as_str(&post, "text_raw")?)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<HashSet<String>>();
        posts
            .into_iter()
            .flat_map(|post| self.extract_pics_from_post(&post))
            .for_each(|url| {
                pics.insert(url);
            });
        debug!("extracted {} pics from posts", pics.len());
        for pic in pics {
            self.get_pic(&pic).await?;
        }
        Ok(())
    }

    pub async fn generate_html(&self, mut posts: Posts, html_name: &str) -> Result<HTMLPage> {
        debug!("generate html from {} posts", posts.len());
        let mut pic_to_fetch = HashSet::new();
        posts
            .iter_mut()
            .map(|mut post| {
                self.process_post(
                    &mut post,
                    &mut pic_to_fetch,
                    &Path::new((Borrowed(html_name) + "_files").as_ref()),
                )
            })
            .collect::<Result<_>>()?;
        let inner_html = self.html_generator.generate_posts(posts)?;
        let html = self.html_generator.generate_page(&inner_html)?;
        let mut pics = Vec::new();
        for pic in pic_to_fetch {
            let blob = self.get_pic(&pic).await?;
            pics.push(Picture {
                name: pic_url_to_file(&pic).into(),
                blob,
            });
        }
        Ok(HTMLPage { html, pics })
    }

    pub async fn get_web_total_num(&self) -> Result<u64> {
        self.web_fetcher.fetch_fav_total_num().await
    }

    pub async fn get_db_total_num(&self) -> Result<u64> {
        self.persister.query_db_total_num().await
    }
}

// ==================================================================================

// Private functions
impl PostProcessor {
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
            && value_as_str(&post, "text_raw")?.starts_with("该内容请至手机客户端查看")
            && self.web_fetcher.has_mobile_cookie()
        {
            post = self
                .fetch_mobile_page(value_as_str(&post, "mblogid")?)
                .await?;
        } else if post["isLongText"] == true {
            let mblogid = value_as_str(&post, "mblogid")?;
            match self.web_fetcher.fetch_long_text_content(mblogid).await {
                Ok(long_text) => post["text_raw"] = Value::String(long_text),
                Err(Error::ResourceGetFailed(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(post)
    }

    async fn fetch_mobile_page(&self, mblogid: &str) -> Result<Value> {
        let text = self.web_fetcher.fetch_mobile_page(mblogid).await?;
        let Some(start) = text.find("\"status\":") else {
                return Err(Error::MalFormat(format!("malformed mobile post: {text}")));
            };
        let Some(end) = text.find("\"call\"") else {
                return Err(Error::MalFormat(format!("malformed mobile post: {text}")));
            };
        let Some(end) = *&text[..end].rfind(",") else {
                return Err(Error::MalFormat(format!("malformed mobile post: {text}")));
            };
        let mut post = from_str::<Value>(&text[start + 9..end])?;
        let id = value_as_str(&post, "id")?;
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
                        .map(|pic| Ok(value_as_str(&pic, "pid")?))
                        .collect::<Result<Vec<_>>>()?,
                )
                .unwrap();
                post["pic_infos"] = serde_json::to_value(
                    pics.into_iter()
                        .map(|mut pic| {
                            let id = value_as_str(&pic, "pid")?.to_owned();
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
            let bid = value_as_str(&post["retweeted_status"], "bid")?;
            post["retweeted_status"]["mblogid"] = Value::String(bid.to_owned());
            let id = value_as_str(&post["retweeted_status"], "id")?;
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
    }

    async fn get_pic(&self, url: &str) -> Result<Bytes> {
        let url = crate::utils::strip_url_queries(url);
        let res = self.persister.query_img(url).await;
        if let Err(Error::NotInLocal) = res {
            let pic = self.web_fetcher.fetch_pic(url).await?;
            self.persister.insert_img(url, &pic).await?;
            Ok(pic)
        } else {
            Ok(res?)
        }
    }

    fn extract_pics_from_post(&self, post: &Post) -> Vec<String> {
        if let Value::Array(pic_ids) = &post["pic_ids"] {
            if pic_ids.len() > 0 {
                let pic_infos = &post["pic_infos"];
                pic_ids
                    .into_iter()
                    .filter_map(|id| id.as_str())
                    .filter_map(|id| pic_infos[id]["mw2000"]["url"].as_str())
                    .map(|url| url.to_owned())
                    .collect()
            } else {
                Default::default()
            }
        } else {
            Default::default()
        }
    }

    fn extract_emoji_from_text(&self, text: &str) -> Vec<String> {
        EMOJI_EXPR
            .find_iter(text)
            .flat_map(|e| self.emoticon.get(e.as_str()).map(|url| url.into()))
            .collect()
    }

    fn process_post(
        &self,
        post: &mut Post,
        pics: &mut HashSet<String>,
        resource_dir: &Path,
    ) -> Result<()> {
        if post["retweeted_status"].is_object() {
            self.process_post_non_rec(&mut post["retweeted_status"], pics, resource_dir)?;
        }
        self.process_post_non_rec(post, pics, resource_dir)?;
        Ok(())
    }

    fn process_post_non_rec(
        &self,
        post: &mut Post,
        pic_urls: &mut HashSet<String>,
        resource_dir: &Path,
    ) -> Result<()> {
        let urls = self.extract_pics_from_post(post);
        let pic_locs: Vec<_> = urls
            .iter()
            .map(|url| resource_dir.join(pic_url_to_file(url)))
            .collect();
        if !pic_locs.is_empty() {
            post["pics"] = to_value(pic_locs).unwrap();
        }

        urls.into_iter().for_each(|url| {
            pic_urls.insert(url);
        });

        let text_raw = value_as_str(&post, "text_raw")?;
        let url_struct = &post["url_struct"];
        let text = self.trans_text(text_raw, url_struct, pic_urls, resource_dir)?;
        trace!("conv {} to {}", text_raw, &text);
        post["text_raw"] = to_value(text).unwrap();
        if post["user"]["avatar_hd"].is_string() {
            // FIXME: avatar_hd may not exists
            let avatar_url = value_as_str(&post["user"], "avatar_hd")?;
            pic_urls.insert(avatar_url.into());
            let avatar_loc = resource_dir.join(pic_url_to_file(avatar_url));
            post["poster_avatar"] = to_value(avatar_loc).unwrap();
        }

        Ok(())
    }

    fn trans_text(
        &self,
        text: &str,
        url_struct: &Value,
        pic_urls: &mut HashSet<String>,
        pic_folder: &Path,
    ) -> Result<String> {
        let emails_suffixes = EMAIL_EXPR
            .find_iter(text)
            .filter_map(|m| AT_EXPR.find(m.as_str()).map(|m| m.as_str()))
            .collect::<HashSet<_>>();
        let text = NEWLINE_EXPR.replace_all(text, "<br />");
        let text = {
            let res = URL_EXPR
                .find_iter(&text)
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + &text[i..m.start()]
                            + self.trans_url(&text[m.start()..m.end()], url_struct),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };
        let text = {
            let res = AT_EXPR
                .find_iter(&text)
                .filter_map(|m| (!emails_suffixes.contains(m.as_str())).then_some(m))
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + Borrowed(&text[i..m.start()])
                            + self.trans_user(&text[m.start()..m.end()]),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };
        let text = {
            let res = TOPIC_EXPR
                .find_iter(&text)
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + &text[i..m.start()] + self.trans_topic(&text[m.start()..m.end()]),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };
        let text = {
            let res = EMOJI_EXPR
                .find_iter(&text)
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + &text[i..m.start()]
                            + self.trans_emoji(&text[m.start()..m.end()], pic_urls, pic_folder),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };

        Ok(text.to_string())
    }

    fn trans_emoji<'a>(
        &self,
        s: &'a str,
        pic_urls: &mut HashSet<String>,
        pic_folder: &'a Path,
    ) -> Cow<'a, str> {
        if let Some(url) = self.emoticon.get(s) {
            pic_urls.insert(url.into());
            let pic_name = pic_url_to_file(url).to_owned();
            Borrowed(r#"<img class="bk-emoji" alt=""#)
                + s
                + r#"" title=""#
                + s
                + r#"" src=""#
                + Owned(
                    pic_folder
                        .join(pic_name)
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                )
                + r#"" />"#
        } else {
            Borrowed(s)
        }
    }

    fn trans_user<'a>(&self, s: &'a str) -> Cow<'a, str> {
        Borrowed(r#"<a class="bk-user" href="https://weibo.com/n/"#) + &s[1..] + "\">" + s + "</a>"
    }

    fn trans_topic<'a>(&self, s: &'a str) -> Cow<'a, str> {
        Borrowed(r#"<a class ="bk-link" href="https://s.weibo.com/weibo?q="#)
            + s
            + r#"" target="_blank">"#
            + s
            + "</a>"
    }

    fn trans_url<'a>(&self, s: &'a str, url_struct: &Value) -> Cow<'a, str> {
        let mut url_title = Borrowed("网页链接");
        let mut url = Borrowed(s);
        if let Value::Array(url_objs) = url_struct {
            if let Some(obj) = url_objs
                .into_iter()
                .find(|obj| obj["short_url"].is_string() && obj["short_url"].as_str().unwrap() == s)
            {
                assert!(obj["url_title"].is_string() && obj["long_url"].is_string());
                url_title = Owned(obj["url_title"].as_str().unwrap().into());
                url = Owned(obj["long_url"].as_str().unwrap().into());
            }
        }
        Borrowed(r#"<a class="bk-link" target="_blank" href=""#)
            + url
            + r#""><img class="bk-icon-link" src="https://h5.sinaimg.cn/upload/2015/09/25/3/timeline_card_small_web_default.png"/>"#
            + url_title
            + "</a>"
    }
}
