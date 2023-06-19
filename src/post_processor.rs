use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};

use anyhow::Result;
use log::trace;
use regex::Regex;
use serde_json::{to_value, Value};
use urlencoding::encode;

use crate::data::Post;
use crate::utils::{pic_url_to_file, strip_url_queries};

#[derive(Debug)]
pub struct PostProcessor {
    emoticon: HashMap<String, String>,
}

impl PostProcessor {
    pub fn new(emoticon: HashMap<String, String>) -> Self {
        Self { emoticon }
    }

    pub fn process_post(&self, post: &mut Post, pics: &mut HashSet<String>) -> Result<()> {
        let pic_folder = "./weiback_files/";
        if post["retweeted_status"].is_object() {
            self.process_post_non_rec(&mut post["retweeted_status"], pics, pic_folder)?;
        }
        self.process_post_non_rec(post, pics, &pic_folder)?;
        Ok(())
    }

    fn process_post_non_rec(
        &self,
        post: &mut Post,
        pic_urls: &mut HashSet<String>,
        pic_folder: &str,
    ) -> Result<()> {
        if let Value::Array(pic_ids) = post["pic_ids"].take() {
            if pic_ids.len() > 0 {
                let pic_infos = &post["pic_infos"];
                let mut pic_locs = Vec::new();
                for id in pic_ids {
                    let url = strip_url_queries(
                        pic_infos[id.as_str().unwrap()]["mw2000"]["url"]
                            .as_str()
                            .expect("cannot get pic info"),
                    );
                    let name = String::from(pic_folder) + pic_url_to_file(url);
                    pic_locs.push(name);
                    pic_urls.insert(url.into());
                }
                if let Value::Object(obj) = post {
                    obj.insert("pics".into(), to_value(pic_locs).unwrap());
                } else {
                    panic!("unexpected post format")
                }
            }
        }
        let text_raw = post["text_raw"].as_str().unwrap();
        let url_struct = &post["url_struct"];
        let text = self.trans_text(text_raw, url_struct, pic_urls, pic_folder)?;
        trace!("conv {} to {}", text_raw, &text);
        post["text_raw"] = to_value(text).unwrap();
        let avatar_url = post["user"]["avatar_hd"].as_str().unwrap();
        pic_urls.insert(avatar_url.into());
        let avatar_loc = Borrowed(pic_folder) + Borrowed(pic_url_to_file(avatar_url));
        post["poster_avatar"] = to_value(avatar_loc).unwrap();

        Ok(())
    }

    fn trans_text(
        &self,
        text: &str,
        url_struct: &Value,
        pic_urls: &mut HashSet<String>,
        pic_folder: &str,
    ) -> Result<String> {
        let newline_expr = Regex::new("\\n").unwrap();
        let url_expr = Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}").unwrap();
        let at_expr = Regex::new(r#"@[\u4e00-\u9fa5|\uE7C7-\uE7F3|\w_\-·]+"#).unwrap();
        let emoji_expr = Regex::new(r#"(\[.*?\])"#).unwrap();
        let email_expr =
            Regex::new(r#"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}"#)
                .unwrap();
        let topic_expr = Regex::new(r#"#([^#]+)#"#).unwrap();

        let emails_suffixes = email_expr
            .find_iter(text)
            .filter_map(|m| at_expr.find(m.as_str()).map(|m| m.as_str()))
            .collect::<HashSet<_>>();
        let text = newline_expr.replace_all(text, "<br />");
        let text = {
            let res = url_expr
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
            let res = at_expr
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
            let res = topic_expr
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
            let res = emoji_expr
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
        pic_folder: &'a str,
    ) -> Cow<'a, str> {
        if let Some(url) = self.emoticon.get(s) {
            pic_urls.insert(url.into());
            let pic_name = pic_url_to_file(url).to_owned();
            Borrowed(r#"<img class="bk-emoji" alt=""#)
                + Borrowed(s)
                + Borrowed(r#"" title=""#)
                + Borrowed(s)
                + Borrowed(r#"" src=""#)
                + Borrowed(pic_folder)
                + Owned(pic_name)
                + Borrowed(r#"" />"#)
        } else {
            Borrowed(s)
        }
    }

    fn trans_user<'a>(&self, s: &'a str) -> Cow<'a, str> {
        Borrowed(r#"<a class="bk-user" href="https://weibo.com/n/"#)
            + Borrowed(&s[1..])
            + Borrowed("\">")
            + Borrowed(s)
            + Borrowed("</a>")
    }

    fn trans_topic<'a>(&self, s: &'a str) -> Cow<'a, str> {
        Borrowed(r#"<a class ="bk-link" href="https://s.weibo.com/weibo?q="#)
            + encode(s)
            + Borrowed(r#"" target="_blank">"#)
            + Borrowed(s)
            + Borrowed("</a>")
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
            + Borrowed(
                r#""><img class="bk-icon-link" src="https://h5.sinaimg.cn/upload/2015/09/25/3/timeline_card_small_web_default.png"/>"#,
            )
            + url_title
            + Borrowed("</a>")
    }
}

#[cfg(test)]
mod post_processor {
    use super::*;
    #[tokio::test]
    async fn trans_text() {
        let pcr = PostProcessor::new(HashMap::new());
        let text = &["教纳德拉做一个产品。\n\n产品可以理解为软件行业的ODM，客户可以象订购Dell电脑一样订购定制化的软件。\n\n当然这个本身不稀奇，稀奇的是，微软只出需求管理、项目管理、和代码审查人员，并不出开发人员和测试人员。\n\n软件要求开源。管理人员整理好需求后面向GitHub开发者征求开发人员，管理人员可以根据 ​​​", "一种可以作恶的安全感//@闫昊佳:现实中接触过不少被“霸凌”的案例，轻重不一，共同点是：其中的霸凌者们会精准识别出“谁可以被霸凌”且自己大概率不会受到惩罚。一个人的主体性塑造和自我边界建立太重要了，尤其对于未成年人。", "//@李富强Jason:转发微博" , "Redis与作者antirez的故事\nhttp://t.cn/A6NnfWeF", "除了折腾内核从没用过[二哈]", "#如何控制自己的情绪# \n\n最近真的很烦恼。我是一个很容易被别人挑拨，情绪控制不住的人，本来想好好解决一件事，被别人一挑就啥话都说出来。感觉自己很幼稚，怎么才能控制住情绪，能更好的处理人际关系呢？\n\n答：\n\n如何控制好自己的情绪？\n\n那就要了解情绪是如何产生的。\n\n心理学上有一个情绪abc ​​"];
        let mut set = HashSet::new();
        text.into_iter().for_each(|s| {
            println!(
                "{}",
                pcr.trans_text(s, &Value::Null, &mut set, "resources/")
                    .unwrap()
            )
        });
    }
}
