use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use log::{debug, error};
use serde_json::Value;
use tera::{Context, Tera};

use crate::error::Result;
use crate::exporter::ExportOptions;
use crate::models::{PictureMeta, Post};
use crate::utils::{
    AT_EXPR, EMAIL_EXPR, EMOJI_EXPR, NEWLINE_EXPR, TOPIC_EXPR, URL_EXPR, url_to_filename,
};

pub fn create_tera(template_path: &Path) -> Result<Tera> {
    let mut path = template_path
        .to_str()
        .expect("template path cannot convert to str")
        .to_owned();
    path.push_str("/*.html");
    debug!("init tera from template: {}", path);
    let mut templates = match Tera::new(&path) {
        Ok(t) => t,
        Err(e) => {
            error!("tera template parse err: {e}");
            panic!("tera template parse err: {e}")
        }
    };
    templates.autoescape_on(Vec::new());
    Ok(templates)
}

#[derive(Debug, Clone)]
pub struct HTMLGenerator {
    templates: Tera,
    emoji_map: Option<HashMap<String, String>>,
}

impl HTMLGenerator {
    pub fn new(templates: Tera) -> Self {
        Self {
            templates,
            emoji_map: None,
        }
    }

    pub fn set_emoji(&mut self, emoji_map: HashMap<String, String>) {
        self.emoji_map = Some(emoji_map);
    }

    pub fn generate_posts(&self, _posts: &[Post], options: &ExportOptions) -> Result<String> {
        let context = Context::new();
        // context.insert("posts", &posts)
        // TODO
        // let context = Context::from_value(posts)?;
        let html = self.templates.render("posts.html", &context)?;
        Ok(html)
    }

    pub fn generate_page(&self, posts: &str) -> Result<String> {
        let mut context = Context::new();
        context.insert("html", &posts);
        let html = self.templates.render("page.html", &context)?;
        Ok(html)
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
        if let Some(url) = self.emoji_map.as_ref().unwrap().get(s) {
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
