pub mod html_generator;

use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::{HashMap, HashSet},
    path::Path,
};

use lazy_static::lazy_static;
use bytes::Bytes;
use regex::Regex;
use serde_json::{Value, to_value};
use weibosdk_rs::{WeiboAPI, emoji};

use crate::{
    error::Result,
    models::{
        Post,
        picture::{Picture, PictureMeta},
    },
    ports::{Storage, TaskOptions},
};
use html_generator::HTMLGenerator;

lazy_static! {
    static ref NEWLINE_EXPR: Regex = Regex::new(r"\n").unwrap();
    static ref URL_EXPR: Regex =
        Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}").unwrap();
    static ref AT_EXPR: Regex =
        Regex::new(r"@[\\u4e00-\\u9fa5|\\uE7C7-\\uE7F3|\\w_\\-·]+").unwrap();
    static ref EMOJI_EXPR: Regex = Regex::new(r"(\\[.*?\\])").unwrap();
    static ref EMAIL_EXPR: Regex =
        Regex::new(r"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\\.)+[A-Za-z]{2,6}").unwrap();
    static ref TOPIC_EXPR: Regex = Regex::new(r"#([^#]+)#").unwrap();
}

#[derive(Debug, Clone)]
pub struct PostProcesser<'a, W: WeiboAPI, S: Storage> {
    api_client: &'a W,
    storage: &'a S,
    emoji_map: Option<HashMap<String, String>>,
}

impl<'a, W: WeiboAPI, S: Storage> PostProcesser<'a, W, S> {
    pub fn new(api_client: &'a W, storage: &'a S) -> Self {
        Self {
            api_client,
            storage,
            emoji_map: None,
        }
    }

    pub async fn process(&self, post: &mut Post, options: &TaskOptions) -> Result<()> {
        let mut pictures = HashSet::new();
        let emoji_urls = self.extract_emoji_urls(&post.text);
        let pic_urls = self.extract_pic_urls(post, &options);

        for url in emoji_urls {
            pictures.insert(PictureMeta::other(url.to_string()));
        }
        for url in pic_urls {
            pictures.insert(PictureMeta::in_post(url.to_string(), post.id));
        }

        for pic in pictures {
            self.storage.save_picture(&pic).await?;
        }

        if post.is_long_text {
            if let Ok(long_text_post) = self.api_client.get_long_text(post.id).await {
                post.text = long_text_post;
            }
        }
        Ok(())
    }

    pub async fn generate_html(&self, posts: &[Post], options: &TaskOptions) -> Result<String> {
        let mut pic_to_fetch = HashSet::new();
        let posts_context = posts
            .iter()
            .map(|post| {
                let mut post = post.clone();
                let pic_urls = self.extract_pic_urls(&post, &options);
                let emoji_urls = self.extract_emoji_urls(&post.text);

                let pic_locs: Vec<_> = pic_urls
                    .iter()
                    .map(|url| {
                        let pic = PictureMeta::in_post(url.to_string(), post.id);
                        let file_name = pic.get_file_name();
                        pic_to_fetch.insert(pic);
                        Path::new("./resources").join(file_name)
                    })
                    .collect();

                emoji_urls.iter().for_each(|url| {
                    pic_to_fetch.insert(PictureMeta::other(url.to_string()));
                });

                let new_text = self
                    .trans_text(&post, Path::new("./resources"))
                    .unwrap_or_default();
                post.text = new_text;

                let mut context = to_value(post).unwrap();
                if !pic_locs.is_empty() {
                    context["pics"] = to_value(pic_locs).unwrap();
                }
                context
            })
            .collect::<Vec<_>>();

        for pic in pic_to_fetch {
            self.storage.save_picture(&pic).await?;
        }

        let inner_html = HTMLGenerator::generate_posts(posts_context)?;
        let html = HTMLGenerator::generate_page(&inner_html)?;
        Ok(html)
    }

    fn extract_emojis(text: &str) -> Vec<&str> {
        EMOJI_EXPR.find_iter(text).map(|e| e.as_str()).collect()
    }

    fn extract_emoji_urls(&self, text: &str) -> Vec<&str> {
        let emojis = Self::extract_emojis(text);
        emojis
            .into_iter()
            .flat_map(|e| self.emoji_map.as_ref().map(|m| m.get(e)))
            .filter_map(|i| i.map(|s| s.as_str()))
            .collect()
    }

    fn pic_ids_to_urls(
        pic_ids: &'a [String],
        pic_infos: &'a HashMap<String, Value>,
        quality: &'a str,
    ) -> Vec<&'a str> {
        pic_ids
            .iter()
            .filter_map(|id| {
                pic_infos
                    .get(id)
                    .map(|v| v[quality]["url"].as_str())
                    .flatten()
            })
            .collect()
    }

    fn extract_pic_urls(&self, post: &'a Post, options: &TaskOptions) -> Vec<&str> {
        let mut pic_vec = post
            .pic_ids
            .as_ref()
            .map(|pic_ids| {
                post.pic_infos.as_ref().map(|pic_infos| {
                    Self::pic_ids_to_urls(pic_ids, pic_infos, options.pic_quality.into())
                })
            })
            .flatten()
            .unwrap_or_default();
        if let Some(retweeted_post) = &post.retweeted_status {
            let mut retweeted_pic_vec = self.extract_pic_urls(retweeted_post, options);
            pic_vec.append(&mut retweeted_pic_vec);
        }
        pic_vec
    }

    async fn get_pictures(&self, post: &Post, options: &TaskOptions) -> Result<Vec<Picture>> {
        let mut pic_urls = self.extract_pic_urls(post, options);
        let mut emoji_urls = self.find_emoji_urls(&post.text);
        pic_urls.append(&mut emoji_urls);
        let mut pics = Vec::new();
        for url in pic_urls {
            let blob = self.api_client.download_picture(&url).await?;
            pics.push(Picture {
                meta: PictureMeta::InPost {
                    url: url.to_string(),
                    post_id: post.id,
                },
                blob,
            });
        }
        Ok(pics)
    }

    fn trans_text(&self, post: &Post, pic_folder: &Path) -> Result<String> {
        let emails_suffixes = EMAIL_EXPR
            .find_iter(&post.text)
            .filter_map(|m| AT_EXPR.find(m.as_str()).map(|m| m.as_str()))
            .collect::<HashSet<_>>();
        let text = NEWLINE_EXPR.replace_all(&post.text, "<br />");
        let text = {
            let res = URL_EXPR
                .find_iter(&text)
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + &text[i..m.start()] + self.trans_url(post, &text[m.start()..m.end()]),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };
        let text = {
            let res = AT_EXPR
                .find_iter(&text)
                .filter(|m| !emails_suffixes.contains(m.as_str()))
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + Borrowed(&text[i..m.start()])
                            + Self::trans_user(&text[m.start()..m.end()]),
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
                        acc + &text[i..m.start()] + Self::trans_topic(&text[m.start()..m.end()]),
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
                            + self.trans_emoji(&text[m.start()..m.end()], pic_folder),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };
        Ok(text.to_string())
    }

    fn trans_emoji(&self, s: &'a str, pic_folder: &'a Path) -> Cow<'a, str> {
        if let Some(url) = self.emoji_map.unwrap().get(s) {
            let pic = PictureMeta::other(url.to_string());
            let pic_name = pic.get_file_name();
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

    fn trans_user(s: &str) -> Cow<str> {
        Borrowed(r#"<a class="bk-user" href="https://weibo.com/n/"#) + &s[1..] + "\">" + s + "</a>"
    }

    fn trans_topic(s: &str) -> Cow<str> {
        Borrowed(r#"<a class ="bk-link" href="https://s.weibo.com/weibo?q="#)
            + s
            + r#"" target="_blank">"#
            + s
            + "</a>"
    }

    fn trans_url(&self, post:&Post, s: &'a str) -> Cow<'a, str> {
        let mut url_title = Borrowed("网页链接");
        let mut url = Borrowed(s);
        if let Some(Value::Array(url_objs)) = post.url_struct.as_ref() {
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
