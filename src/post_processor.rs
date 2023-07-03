use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::path::Path;

use lazy_static::lazy_static;
use log::{debug, trace};
use regex::Regex;
use serde_json::{to_value, Value};

use crate::data::{Post, Posts};
use crate::error::Result;
use crate::exporter::{HTMLPage, Picture};
use crate::html_generator::HTMLGenerator;
use crate::resource_manager::ResourceManager;
use crate::utils::pic_url_to_file;

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
    resource_manager: ResourceManager,
    emoticon: HashMap<String, String>,
}

impl PostProcessor {
    pub fn new(resource_manager: ResourceManager) -> Self {
        Self {
            html_generator: HTMLGenerator::new(),
            resource_manager,
            emoticon: Default::default(),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        debug!("initing...");
        self.emoticon = self.resource_manager.get_emoticon().await?;
        self.resource_manager.init().await?;
        Ok(())
    }

    pub async fn get_fav_posts_from_web(&self, uid: &str, page: u32) -> Result<Posts> {
        self.resource_manager
            .get_fav_posts_from_web(uid, page)
            .await
    }

    pub async fn get_fav_post_from_db(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Posts> {
        self.resource_manager
            .get_fav_post_from_db(range, reverse)
            .await
    }

    pub async fn save_post_pictures(&self, posts: Posts) -> Result<()> {
        debug!("save pictures of posts to db...");
        let mut pics = posts
            .data
            .iter()
            .flat_map(|ref post| self.extract_emoji_from_text(&post["text_raw"].as_str().unwrap()))
            .collect::<HashSet<_>>();
        posts
            .into_iter()
            .flat_map(|post| self.extract_pics_from_post(&post))
            .for_each(|url| {
                pics.insert(url);
            });
        debug!("extracted {} pics from posts", pics.len());
        for pic in pics {
            self.resource_manager.get_pic(&pic).await?;
        }
        Ok(())
    }

    pub async fn generate_html(&self, mut posts: Posts, html_name: &str) -> Result<HTMLPage> {
        debug!("generate html from {} posts", posts.len());
        let mut pic_to_fetch = HashSet::new();
        posts
            .data
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
            let blob = self.resource_manager.get_pic(&pic).await?;
            pics.push(Picture {
                name: pic_url_to_file(&pic).into(),
                blob,
            });
        }
        Ok(HTMLPage { html, pics })
    }

    fn extract_pics_from_post(&self, post: &Post) -> Vec<String> {
        if let Value::Array(pic_ids) = &post["pic_ids"] {
            if pic_ids.len() > 0 {
                let pic_infos = &post["pic_infos"];
                pic_ids
                    .into_iter()
                    .map(|id| {
                        pic_infos[id.as_str().unwrap()]["mw2000"]["url"]
                            .as_str()
                            .expect("url of pics should be str")
                            .into()
                    })
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

        let text_raw = post["text_raw"].as_str().unwrap();
        let url_struct = &post["url_struct"];
        let text = self.trans_text(text_raw, url_struct, pic_urls, resource_dir)?;
        trace!("conv {} to {}", text_raw, &text);
        post["text_raw"] = to_value(text).unwrap();
        if post["user"]["avatar_hd"].is_string() {
            let avatar_url = post["user"]["avatar_hd"].as_str().unwrap();
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
            if let Some(obj) = url_objs.into_iter().find(|obj| {
                obj["short_url"]
                    .as_str()
                    .expect("there should be 'short url' in url_struct")
                    == s
            }) {
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

    pub async fn get_web_total_num(&self) -> Result<u64> {
        self.resource_manager.get_web_total_num().await
    }

    pub async fn get_db_total_num(&self) -> Result<u64> {
        self.resource_manager.get_db_total_num().await
    }
}
