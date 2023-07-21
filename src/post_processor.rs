use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::path::Path;

use bytes::Bytes;
use futures::future::join_all;
use lazy_static::lazy_static;
use log::{debug, trace, warn};
use regex::Regex;
use serde_json::{to_value, Value};

use crate::data::{Post, Posts};
use crate::error::{Error, Result};
use crate::exporter::{HTMLPage, Picture};
use crate::html_generator::HTMLGenerator;
use crate::persister::Persister;
use crate::utils::pic_url_to_file;
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

const IMG_TYPES: &[&[&'static str; 6]; 3] = &[
    &[
        "thunmnail",
        "bmiddle",
        "large",
        "original",
        "largest",
        "mw2000",
    ],
    &[
        "large",
        "original",
        "bmiddle",
        "largest",
        "thumbnail",
        "mw2000",
    ],
    &[
        "mw2000",
        "largest",
        "original",
        "large",
        "bmiddle",
        "thumbnail",
    ],
];

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

    pub async fn unfavorite_fav_posts(&self, range: RangeInclusive<u32>) -> Result<()> {
        let limit = (range.end() - range.start()) + 1;
        let offset = *range.start() - 1;
        let ids = self
            .persister
            .query_posts_to_unfavorite(limit, offset)
            .await?;
        debug!("load {} posts to unfavorite", ids.len());
        for id in ids {
            self.web_fetcher.unfavorite_post(id).await?;
            self.persister.mark_post_unfavorited(id).await?;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok(())
    }

    pub async fn download_fav_posts(
        &self,
        uid: &str,
        page: u32,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<usize> {
        let posts = self.web_fetcher.fetch_posts_meta(uid, page).await?;
        let result = posts.len();
        self.persist_posts(posts, with_pic, image_definition)
            .await?;
        Ok(result)
    }

    pub async fn load_fav_posts_from_db(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Posts> {
        let limit = (range.end() - range.start()) + 1;
        let offset = *range.start() - 1;
        self.persister.query_posts(limit, offset, reverse).await
    }

    pub async fn generate_html(
        &self,
        mut posts: Posts,
        html_name: &str,
        image_definition: u8,
    ) -> Result<HTMLPage> {
        debug!("generate html from {} posts", posts.len());
        let mut pic_to_fetch = HashSet::new();
        posts
            .iter_mut()
            .map(|mut post| {
                self.process_post(
                    &mut post,
                    &mut pic_to_fetch,
                    &Path::new((Borrowed(html_name) + "_files").as_ref()),
                    image_definition,
                )
            })
            .collect::<Result<_>>()?;
        let inner_html = self.html_generator.generate_posts(posts)?;
        let html = self.html_generator.generate_page(&inner_html)?;
        let mut pics = Vec::new();
        for pic in pic_to_fetch {
            if let Some(blob) = self.get_pic(&pic).await? {
                pics.push(Picture {
                    name: pic_url_to_file(&pic).into(),
                    blob,
                });
            }
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
    async fn persist_posts(
        &self,
        posts: Posts,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<()> {
        let posts = join_all(
            posts
                .into_iter()
                .map(|post| async { self.preprocess_post(post).await }),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
        if with_pic {
            self.persist_post_pictures(&posts, image_definition).await?;
        }
        Ok(())
    }

    async fn persist_post_pictures(&self, posts: &Posts, image_definition: u8) -> Result<()> {
        debug!("save pictures of posts to db...");
        let mut pics = posts
            .iter()
            .map(|ref post| {
                Ok(self.extract_emoji_from_text(
                    (Borrowed(value_as_str(&post, "text_raw")?)
                        + post["retweeted_status"]["text_raw"]
                            .as_str()
                            .unwrap_or_default())
                    .as_ref(),
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<HashSet<String>>();
        posts
            .into_iter()
            .flat_map(|post| self.extract_pics_from_post(post, image_definition))
            .for_each(|url| {
                pics.insert(url);
            });
        debug!("extracted {} pics from posts", pics.len());
        for pic in pics {
            self.get_pic(&pic).await?;
        }
        Ok(())
    }

    async fn preprocess_post(&self, post: Post) -> Result<Post> {
        let id = value_as_i64(&post, "id")?;
        match self.persister.query_post(id).await {
            Ok(post) => Ok(post),
            Err(Error::NotInLocal) => {
                let mut post = self.preprocess_post_non_rec(post).await?;
                self.persister.insert_post(&post).await?;

                if let Some(id) = post["retweeted_status"]["id"].as_i64() {
                    match self.persister.query_post(id).await {
                        Ok(retweet) => {
                            post["retweeted_status"] = retweet;
                            return Ok(post);
                        }
                        Err(Error::NotInLocal) => {
                            let mut retweet = self
                                .preprocess_post_non_rec(post["retweeted_status"].take())
                                .await?;
                            if let Value::Array(url_struct) = post["url_struct"].take() {
                                let mut url_struct = url_struct
                                    .into_iter()
                                    .map(|st| Ok((value_as_str(&st, "short_url")?.to_owned(), st)))
                                    .collect::<Result<HashMap<String, Value>>>()?;
                                retweet["url_struct"] = Value::Array(
                                    extract_urls(value_as_str(&retweet, "text_raw")?)
                                        .into_iter()
                                        .filter_map(|url| url_struct.remove(url))
                                        .collect(),
                                );
                                post["url_struct"] = Value::Array(
                                    extract_urls(value_as_str(&post, "text_raw")?)
                                        .into_iter()
                                        .filter_map(|url| url_struct.remove(url))
                                        .collect(),
                                )
                            }
                            if post["page_info"].is_object() {
                                retweet["page_info"] = post["page_info"].take();
                            }
                            post["retweeted_status"] = retweet;
                        }
                        e => return e,
                    }
                }

                Ok(post)
            }
            e => e,
        }
    }

    async fn preprocess_post_non_rec(&self, mut post: Post) -> Result<Post> {
        if !post["user"]["id"].is_number() {
            if value_as_str(&post, "text_raw")?.starts_with("该内容请至手机客户端查看")
            {
                post["client_only"] = Value::Bool(true);
            }
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

    async fn get_pic(&self, url: &str) -> Result<Option<Bytes>> {
        let url = crate::utils::strip_url_queries(url);
        let res = self.persister.query_img(url).await;
        if let Err(Error::NotInLocal) = res {
            let pic = match self.web_fetcher.fetch_pic(url).await {
                Ok(pic) => pic,
                Err(err) => {
                    warn!("pic get failed {}", err);
                    return Ok(None);
                }
            };
            self.persister.insert_img(url, &pic).await?;
            Ok(Some(pic))
        } else {
            Ok(Some(res?))
        }
    }

    fn extract_pics_from_post(&self, post: &Post, image_definition: u8) -> Vec<String> {
        let mut res = self.extract_pics_from_post_non_rec(post, image_definition);
        if post["retweeted_status"].is_object() {
            res.append(
                &mut self
                    .extract_pics_from_post_non_rec(&post["retweeted_status"], image_definition),
            );
        }
        res
    }

    fn extract_pics_from_post_non_rec(&self, post: &Post, image_definition: u8) -> Vec<String> {
        if let Value::Array(pic_ids) = &post["pic_ids"] {
            if !pic_ids.is_empty() {
                let pic_infos = &post["pic_infos"];
                let mut pic_urls: Vec<_> = pic_ids
                    .iter()
                    .filter_map(|id| id.as_str())
                    .filter_map(|id| self.select_pic_url(&pic_infos[id], image_definition))
                    .map(|url| url.to_owned())
                    .collect();
                if let Some(avatar_url) = self.get_avatar_url(post, image_definition) {
                    pic_urls.push(avatar_url.to_owned());
                }
                pic_urls
            } else {
                Default::default()
            }
        } else {
            Default::default()
        }
    }

    fn select_pic_url<'a>(&self, pic_info: &'a Value, image_definition: u8) -> Option<&'a str> {
        if pic_info.is_null() {
            return None;
        }
        IMG_TYPES[image_definition as usize]
            .iter()
            .skip_while(|t| pic_info[t].is_string())
            .next()
            .and_then(|t| pic_info[t]["url"].as_str())
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
        image_definition: u8,
    ) -> Result<()> {
        if post["retweeted_status"].is_object() {
            self.process_post_non_rec(
                &mut post["retweeted_status"],
                pics,
                resource_dir,
                image_definition,
            )?;
        }
        self.process_post_non_rec(post, pics, resource_dir, image_definition)?;
        Ok(())
    }

    fn get_avatar_url<'a>(&self, post: &'a Value, image_definition: u8) -> Option<&'a str> {
        let avatar_type = match image_definition {
            0 => "profile_image_url",
            1 => "avatar_large",
            2 => "avatar_hd",
            _ => unreachable!(),
        };
        post["user"][avatar_type].as_str()
    }

    fn process_post_non_rec(
        &self,
        post: &mut Post,
        pic_urls: &mut HashSet<String>,
        resource_dir: &Path,
        image_definition: u8,
    ) -> Result<()> {
        let urls = self.extract_pics_from_post_non_rec(post, image_definition);
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

        let text_raw = value_as_str(post, "text_raw")?;
        let url_struct = &post["url_struct"];
        let text = self.trans_text(text_raw, url_struct, pic_urls, resource_dir)?;
        trace!("conv {} to {}", text_raw, text);
        post["text_raw"] = to_value(text).unwrap();
        if let Some(avatar_url) = self.get_avatar_url(post, image_definition) {
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
                .iter()
                .find(|obj| obj["short_url"].is_string() && obj["short_url"].as_str().unwrap() == s)
            {
                assert!(obj["url_title"].is_string() && obj["long_url"].is_string());
                url_title = Owned(obj["url_title"].as_str().unwrap().into());
                url = Owned(obj["long_url"].as_str().unwrap().into());
            }
        }
        Borrowed(r#"<a class="bk-link" target="_blank" href=""#)
            + url
            + "\"><img class=\"bk-icon-link\" src=\"https://h5.sinaimg.cn/upload/2015/09/25/3/\
               timeline_card_small_web_default.png\"/>"
            + url_title
            + "</a>"
    }
}

fn value_as_str<'a>(v: &'a Value, property: &'a str) -> Result<&'a str> {
    v[property].as_str().ok_or(Error::MalFormat(format!(
        "property {} of {} cannot convert to str",
        property, v
    )))
}

fn value_as_i64<'a>(v: &'a Value, property: &'a str) -> Result<i64> {
    v[property].as_i64().ok_or(Error::MalFormat(format!(
        "property {} of {} cannot convert to i64",
        property, v
    )))
}

fn extract_urls(text: &str) -> Vec<&str> {
    URL_EXPR.find_iter(text).map(|m| m.as_str()).collect()
}
