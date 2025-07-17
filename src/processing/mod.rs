pub mod html_generator;

use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::{HashMap, HashSet},
    f64::consts::E,
    path::Path,
};

use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{Value, to_value};
use weibosdk_rs::{WeiboAPI, emoji, long_text};

use crate::error::Result;
use crate::models::{
    Post,
    picture::{Picture, PictureMeta},
};
use crate::ports::{ExportOptions, PictureDefinition, Storage, TaskOptions};
use crate::utils::{pic_url_to_file, pic_url_to_id};
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

    pub async fn process(&self, posts: Vec<Post>, options: &TaskOptions) -> Result<()> {
        let pic_metas = self.extract_pic_metas(&posts, options.pic_quality);

        for meta in pic_metas {
            self.save_picture(meta).await?;
        }

        for mut post in posts {
            if post.is_long_text
                && let Ok(long_text) = self.api_client.get_long_text(post.id).await
            {
                post.text = long_text
            }
            self.storage.save_post(&post).await?;
        }
        Ok(())
    }

    pub async fn generate_html(&self, posts: &[Post], options: &ExportOptions) -> Result<String> {
        let pic_metas = self.extract_pic_metas(posts, options.pic_quality);
        let pic = pic_metas
            .into_iter()
            .map(|m| self.load_picture_from_local(m));
        // TODO: tackle errs
        let (pics, _): (Vec<_>, Vec<_>) = futures::future::join_all(pic)
            .await
            .into_iter()
            .partition_result();
        todo!()
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

    fn extract_in_post_pic_urls(&self, post: &'a Post, definition: PictureDefinition) -> Vec<&str> {
        let mut pic_vec = post
            .pic_ids
            .as_ref()
            .map(|pic_ids| {
                post.pic_infos
                    .as_ref()
                    .map(|pic_infos| Self::pic_ids_to_urls(pic_ids, pic_infos, definition.into()))
            })
            .flatten()
            .unwrap_or_default();
        post.user.as_ref().map(|user| pic_vec.push(&user.avatar_hd));
        if let Some(retweeted_post) = &post.retweeted_status {
            let mut retweeted_pic_vec = self.extract_in_post_pic_urls(retweeted_post, definition);
            pic_vec.append(&mut retweeted_pic_vec);
            retweeted_post
                .user
                .as_ref()
                .map(|user| pic_vec.push(&user.avatar_hd));
        }
        pic_vec
    }

    async fn save_picture(&self, pic_meta: PictureMeta) -> Result<()> {
        if let Some(_) = self.storage.get_picture(pic_meta.url()).await? {
            Ok(())
        } else {
            let blob = self.api_client.download_picture(pic_meta.url()).await?;
            let pic = Picture {
                meta: pic_meta,
                blob: blob,
            };
            self.storage.save_picture(&pic).await?;
            Ok(())
        }
    }

    async fn load_picture_from_local(&self, pic_meta: PictureMeta) -> Result<Option<Picture>> {
        Ok(self
            .storage
            .get_picture(pic_meta.url())
            .await?
            .map(|blob| Picture {
                meta: pic_meta,
                blob,
            }))
    }

    async fn load_picture_from_local_or_server(&self, pic_meta: PictureMeta) -> Result<Picture> {
        if let Some(blob) = self.storage.get_picture(pic_meta.url()).await? {
            Ok(Picture {
                meta: pic_meta,
                blob,
            })
        } else {
            let blob = self.api_client.download_picture(pic_meta.url()).await?;
            let pic = Picture {
                meta: pic_meta,
                blob: blob,
            };
            self.storage.save_picture(&pic).await?;
            Ok(pic)
        }
    }

    fn extract_pic_metas(
        &self,
        posts: &[Post],
        definition: PictureDefinition,
    ) -> HashSet<PictureMeta> {
        let mut pic_metas: HashSet<PictureMeta> = posts
            .into_iter()
            .flat_map(|post| {
                self.extract_in_post_pic_urls(post, definition)
                    .into_iter()
                    .map(|url| PictureMeta::in_post(url.to_string(), post.id))
            })
            .collect();
        let emoji_metas = posts.into_iter().flat_map(|post| {
            self.extract_emoji_urls(&post.text)
                .into_iter()
                .map(|url| PictureMeta::other(url.to_string()))
        });
        pic_metas.extend(emoji_metas);
        pic_metas
    }

    async fn get_pictures(
        &self,
        posts: &[Post],
        definition: PictureDefinition,
    ) -> Result<Vec<Picture>> {
        let pic_metas = self.extract_pic_metas(posts, definition);
        let mut pics = Vec::new();
        for metas in pic_metas {
            pics.push(self.load_picture_from_local_or_server(metas).await?);
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
                            + self
                                .trans_emoji(&text[m.start()..m.end()], pic_folder)
                                .unwrap(),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };
        Ok(text.to_string())
    }

    fn trans_emoji(&self, s: &'a str, pic_folder: &'a Path) -> Result<Cow<'a, str>> {
        if let Some(url) = self.emoji_map.as_ref().unwrap().get(s) {
            let pic = PictureMeta::other(url.to_string());
            let pic_name = pic_url_to_file(pic.url())?;
            Ok(Borrowed(r#"<img class="bk-emoji" alt=""#)
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
                + r#"" />"#)
        } else {
            Ok(Borrowed(s))
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

    fn trans_url(&self, post: &Post, s: &'a str) -> Cow<'a, str> {
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
