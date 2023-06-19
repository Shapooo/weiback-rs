use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::Result;
use chrono;
use log::{debug, info, trace};
use regex::Regex;
use serde_json::{to_value, Value};
use tokio::time::sleep;
use urlencoding::encode;

use crate::config::Config;
use crate::data::Post;
use crate::exporter::{Exporter, HTMLPage, Picture};
use crate::generator::HTMLGenerator;
use crate::persister::Persister;
use crate::resource_manager::ResourceManager;
use crate::utils::{pic_url_to_file, strip_url_queries};
use crate::web_fetcher::WebFetcher;

#[derive(Debug)]
pub struct TaskHandler {
    resource_manager: ResourceManager,
    generator: HTMLGenerator,
    exporter: Exporter,
    config: Config,
    emoticon: HashMap<String, String>,
}

impl TaskHandler {
    pub async fn build(config: Config) -> Result<Self> {
        let fetcher = WebFetcher::build(
            config.web_cookie.clone(),
            (!config.mobile_cookie.is_empty()).then_some(config.mobile_cookie.clone()),
        );
        let persister = Persister::build(&config.db).await?;
        let resource_manager = ResourceManager::build(fetcher, persister);
        let emoticon = resource_manager.get_emoticon().await?;
        Ok(TaskHandler {
            resource_manager,
            generator: HTMLGenerator::new(),
            exporter: Exporter::new(),
            config,
            emoticon,
        })
    }

    pub async fn download(&self, range: RangeInclusive<u64>) -> Result<()> {
        self.download_posts(range, false, false).await
    }

    pub async fn download_with_pic(&self, range: RangeInclusive<u64>) -> Result<()> {
        self.download_posts(range, true, false).await
    }

    pub async fn download_with_pic_and_export(&self, range: RangeInclusive<u64>) -> Result<()> {
        self.download_posts(range, true, true).await
    }

    async fn download_posts(
        &self,
        mut range: RangeInclusive<u64>,
        with_pic: bool,
        export: bool,
    ) -> Result<()> {
        if range.start() == &0 {
            range = RangeInclusive::new(1, *range.end());
        }

        info!("pages download range is {range:?}");
        let mut total_posts_sum = 0;
        let mut pic_to_fetch: HashSet<String> = HashSet::new();
        let mut html = String::new();
        for page in range {
            let posts_meta = self
                .resource_manager
                .get_fav_posts_from_web(self.config.uid.as_str(), page)
                .await?;
            let posts_sum = posts_meta.len();
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }

            if with_pic {
                for mut post in posts_meta {
                    self.process_post(&mut post, &mut pic_to_fetch)?;
                    if export {
                        html.push_str(self.generator.generate_post(post).await?.as_str());
                    }
                }
            }
            sleep(Duration::from_secs(5)).await;
        }

        if export {
            let mut pics = Vec::new();
            for pic_url in pic_to_fetch {
                let name = pic_url_to_file(&pic_url).into();
                let blob = self.resource_manager.get_pic(&pic_url).await?;
                pics.push(Picture { name, blob });
            }
            let html = self.generator.generate_page(&html).await?;
            let page = HTMLPage { html, pics };
            let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%R"));
            self.exporter
                .export_page(task_name, page, std::path::PathBuf::from("./").as_path())
                .await?;
        }
        info!("fetched {total_posts_sum} posts in total");
        Ok(())
    }

    fn process_post(&self, post: &mut Post, pics: &mut HashSet<String>) -> Result<()> {
        if post["retweeted_status"].is_object() {
            self.process_post_non_rec(&mut post["retweeted_status"], pics)?;
        }
        self.process_post_non_rec(post, pics)?;
        Ok(())
    }

    fn process_post_non_rec(&self, post: &mut Post, pics: &mut HashSet<String>) -> Result<()> {
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
                    // TODO: wrong location
                    let name = String::from("resources/") + pic_url_to_file(url);
                    pic_locs.push(name);
                    pics.insert(url.into());
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
        let text = self.trans_text(text_raw, url_struct, pics)?;
        trace!("conv {} to {}", text_raw, &text);
        post["text_raw"] = to_value(text).unwrap();
        post["poster_avatar"] = to_value(
            Borrowed("resources/")
                + Borrowed(pic_url_to_file(post["user"]["avatar_hd"].as_str().unwrap())),
        )
        .unwrap();

        Ok(())
    }

    fn trans_text(
        &self,
        text: &str,
        url_struct: &Value,
        pic_urls: &mut HashSet<String>,
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
                            + self.trans_emoji(&text[m.start()..m.end()], pic_urls),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };

        Ok(text.to_string())
    }

    fn trans_emoji<'a>(&self, s: &'a str, pic_urls: &mut HashSet<String>) -> Cow<'a, str> {
        if let Some(url) = self.emoticon.get(s) {
            pic_urls.insert(url.into());
            let loc = pic_url_to_file(url).to_owned();
            Borrowed(r#"<img class="bk-emoji" alt=""#)
                + Borrowed(s)
                + Borrowed(r#"" title=""#)
                + Borrowed(s)
                + Borrowed(r#"" src="resources/"#) // TODO wrong loc
                + Owned(loc)
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
mod task_handler {
    use super::*;
    #[tokio::test]
    async fn trans_text() {
        let tsk = TaskHandler::build(Config::new()).await.unwrap();
        let text = &["教纳德拉做一个产品。\n\n产品可以理解为软件行业的ODM，客户可以象订购Dell电脑一样订购定制化的软件。\n\n当然这个本身不稀奇，稀奇的是，微软只出需求管理、项目管理、和代码审查人员，并不出开发人员和测试人员。\n\n软件要求开源。管理人员整理好需求后面向GitHub开发者征求开发人员，管理人员可以根据 ​​​", "一种可以作恶的安全感//@闫昊佳:现实中接触过不少被“霸凌”的案例，轻重不一，共同点是：其中的霸凌者们会精准识别出“谁可以被霸凌”且自己大概率不会受到惩罚。一个人的主体性塑造和自我边界建立太重要了，尤其对于未成年人。", "//@李富强Jason:转发微博" , "Redis与作者antirez的故事\nhttp://t.cn/A6NnfWeF", "除了折腾内核从没用过[二哈]", "#如何控制自己的情绪# \n\n最近真的很烦恼。我是一个很容易被别人挑拨，情绪控制不住的人，本来想好好解决一件事，被别人一挑就啥话都说出来。感觉自己很幼稚，怎么才能控制住情绪，能更好的处理人际关系呢？\n\n答：\n\n如何控制好自己的情绪？\n\n那就要了解情绪是如何产生的。\n\n心理学上有一个情绪abc ​​"];
        let mut set = HashSet::new();
        text.into_iter()
            .for_each(|s| println!("{}", tsk.trans_text(s, &Value::Null, &mut set).unwrap()));
    }
}
