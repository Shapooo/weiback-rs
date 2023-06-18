use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::Result;
use chrono;
use log::{debug, info};
use regex::Regex;
use serde_json::{to_value, Value};
use tokio::time::sleep;

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
            self.process_post_non_rec(post, pics, true)?;
        }
        self.process_post_non_rec(post, pics, false)?;
        Ok(())
    }

    fn process_post_non_rec(
        &self,
        post: &mut Post,
        pics: &mut HashSet<String>,
        is_retweet: bool,
    ) -> Result<()> {
        if let Value::Array(pic_ids) = post["pic_ids"].take() {
            let pic_infos = &post["pic_infos"];
            let mut pic_locs = Vec::new();
            for id in pic_ids {
                let url = strip_url_queries(
                    pic_infos[id.as_str().unwrap()]["mw2000"]["url"]
                        .as_str()
                        .expect("cannot get pic info"),
                );
                let name = String::from("resources/") + pic_url_to_file(url);
                pic_locs.push(name);
                pics.insert(url.into());
            }
            if let Value::Object(obj) = post {
                let key = if is_retweet { "retweet_pics" } else { "pics" };
                obj.insert(key.into(), to_value(pic_locs).unwrap());
            } else {
                panic!("")
            }
        }
        let text_raw = post["text_raw"].as_str().unwrap();
        let topic_struct = &post["topic_struct"];
        let url_struct = &post["url_struct"];
        let text = self.trans_text(text_raw, topic_struct, url_struct, pics)?;
        post["text_raw"] = to_value(text).unwrap();

        Ok(())
    }

    fn trans_text(
        &self,
        text: &str,
        _topic_struct: &Value,
        _url_struct: &Value,
        pic_urls: &mut HashSet<String>,
    ) -> Result<String> {
        let newline_expr = Regex::new("\\n").unwrap();
        let single_quote = Regex::new("&#39;").unwrap();
        let url_expr = Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}").unwrap();
        let at_expr = Regex::new(r#"@[\u4e00-\u9fa5|\uE7C7-\uE7F3|\w_\-·]+"#).unwrap();
        let emoji_expr = Regex::new(r#"(\[.*?\])(?!#)"#).unwrap();
        let email_expr =
            Regex::new(r#"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}"#)
                .unwrap();
        let topic_expr = Regex::new(r#"#([^#]+)#"#).unwrap();

        let text = single_quote.replace_all(text, "'");
        let text = newline_expr.replace_all(&text, "<br />");
        let emails_suffixes = email_expr
            .find_iter(&text)
            .filter_map(|m| at_expr.find(m.as_str()).map(|m| m.as_str()))
            .collect::<HashSet<_>>();
        let text = match at_expr
            .find_iter(&text)
            .filter_map(|m| emails_suffixes.contains(m.as_str()).then_some(m))
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + Borrowed(&text[i..m.start()])
                        + self.trans_user(&text[m.start()..m.end()]),
                    m.end(),
                )
            }) {
            (_, 0) => text,
            (res, _) => res,
        };
        let text = match topic_expr
            .find_iter(&text)
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + &text[i..m.start()] + self.trans_topic(&text[m.start()..m.end()]),
                    m.end(),
                )
            }) {
            (_, 0) => text,
            (res, _) => res,
        };
        let text = match url_expr
            .find_iter(&text)
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + &text[i..m.start()] + self.trans_url(&text[m.start()..m.end()]),
                    m.end(),
                )
            }) {
            (_, 0) => text,
            (res, _) => res,
        };
        let text = match emoji_expr
            .find_iter(&text)
            .fold((Borrowed(""), 0), |(acc, i), m| {
                (
                    acc + &text[i..m.start()]
                        + self.trans_emoji(&text[m.start()..m.end()], pic_urls),
                    m.end(),
                )
            }) {
            (_, 0) => text,
            (res, _) => res,
        };

        Ok(text.to_string())
    }

    fn trans_emoji<'a>(&self, s: &'a str, pic_urls: &mut HashSet<String>) -> Cow<'a, str> {
        if let Some(url) = self.emoticon.get(s) {
            pic_urls.insert(url.into());
            let loc = pic_url_to_file(url).to_owned();
            Borrowed(r#"<img class="bk-emoji" alt="["#)
                + Borrowed(s)
                + Borrowed(r#"]" title="["#)
                + Borrowed(s)
                + Borrowed(r#"]" src="resources/"#)
                + Owned(loc)
                + Borrowed(r#"" />"#)
        } else {
            Borrowed(s)
        }
    }

    fn trans_user<'a>(&self, s: &'a str) -> Cow<'a, str> {
        Borrowed(r#"<a class="bk-user" href=https://weibo.com/n/"#)
            + Borrowed(&s[1..])
            + Borrowed(">")
            + Borrowed(s)
            + Borrowed("</a>")
    }

    fn trans_topic<'a>(&self, _s: &'a str) -> Cow<'a, str> {
        todo!()
    }

    fn trans_url<'a>(&self, _s: &'a str) -> Cow<'a, str> {
        todo!()
    }
}
