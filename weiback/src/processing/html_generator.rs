use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use log::{debug, info};
use once_cell::sync::OnceCell;
use serde_json::Value;
use tera::{Context, Tera};
use weibosdk_rs::emoji::EmojiUpdateAPI;

use super::{pic_id_to_url, process_in_post_pics};
use crate::config::get_config;
use crate::error::{Error, Result};
use crate::models::{PictureDefinition, PictureMeta, Post};
use crate::utils::{
    AT_EXPR, EMAIL_EXPR, EMOJI_EXPR, NEWLINE_EXPR, TOPIC_EXPR, URL_EXPR, url_to_filename,
};

pub fn create_tera(template_path: &Path) -> Result<Tera> {
    let mut path = template_path
        .to_str()
        .ok_or(Error::ConfigError(format!(
            "template path in config cannot convert to str: {template_path:?}"
        )))?
        .to_owned();
    path.push_str("/*.html");
    debug!("init tera from template: {path}");
    let mut templates = Tera::new(&path)?;
    templates.autoescape_on(Vec::new());
    Ok(templates)
}

#[derive(Debug, Clone)]
pub struct HTMLGenerator<E: EmojiUpdateAPI> {
    api_client: E,
    templates: Tera,
    emoji_map: OnceCell<HashMap<String, String>>,
}

impl<E: EmojiUpdateAPI> HTMLGenerator<E> {
    pub fn new(api_client: E, templates: Tera) -> Self {
        Self {
            api_client,
            templates,
            emoji_map: OnceCell::new(),
        }
    }

    fn generate_post(&self, mut post: Post, task_name: &str) -> Result<String> {
        let pic_folder = task_name.to_owned() + "_files";
        let pic_quality = get_config().read()?.picture_definition;
        let in_post_pic_paths = extract_in_post_pic_paths(&post, &pic_folder, pic_quality);

        let mut context = Context::new();
        post.text = self.trans_text(&post, PathBuf::from(pic_folder).as_ref())?;
        context.insert("post", &post);
        context.insert("pics", &in_post_pic_paths);
        let html = self.templates.render("post.html", &context)?;
        Ok(html)
    }

    pub fn generate_page(&self, posts: Vec<Post>, task_name: &str) -> Result<String> {
        info!("Generating page for {} posts", posts.len());
        let posts_html = posts
            .into_iter()
            .map(|p| self.generate_post(p, task_name))
            .collect::<Result<Vec<_>>>()?;
        let posts_html = posts_html.join("");
        let mut context = Context::new();
        context.insert("html", &posts_html);
        let html = self.templates.render("page.html", &context)?;
        info!("Successfully generated page");
        Ok(html)
    }

    fn get_or_try_init_emoji(&self) -> Result<&HashMap<String, String>> {
        Ok(self.emoji_map.get_or_try_init(|| {
            let runtime = tokio::runtime::Handle::current();
            runtime.block_on(async move { self.api_client.emoji_update().await })
        })?)
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

    fn trans_emoji<'a>(&self, s: &'a str, pic_folder: &'a Path) -> Result<Cow<'a, str>> {
        if let Some(url) = self.get_or_try_init_emoji().unwrap().get(s) {
            let pic = PictureMeta::other(url.to_string());
            let pic_name = url_to_filename(pic.url())?;
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
                        .map_err(|e| {
                            Error::FormatError(format!("contain invalid unicode in {e:?}"))
                        })?,
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

    fn trans_url<'a>(&self, post: &Post, s: &'a str) -> Cow<'a, str> {
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

fn extract_in_post_pic_paths(
    post: &Post,
    pic_folder: &str,
    pic_quality: PictureDefinition,
) -> Vec<String> {
    process_in_post_pics(post, |id, pic_infos, _| {
        pic_id_to_url(id, pic_infos, &pic_quality)
            .and_then(|url| url_to_filename(url).ok())
            .and_then(|name| {
                Path::new(pic_folder)
                    .join(name)
                    .to_str()
                    .map(|s| s.to_string())
            })
    })
}
