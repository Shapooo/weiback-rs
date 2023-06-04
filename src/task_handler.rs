use std::time::Duration;

use crate::config::Config;
use crate::fetched_data::FetchedPost;
use crate::fetcher::Fetcher;
use crate::persister::Persister;
use crate::sql_data::{SqlPost, SqlUser};
use anyhow::Result;
use log::{debug, info};
use tokio::time::sleep;

#[derive(Debug)]
pub struct TaskHandler {
    fetcher: Fetcher,
    persister: Persister,
    config: Config,
}

impl TaskHandler {
    pub fn build(config: Config) -> Result<Self> {
        let fetcher = Fetcher::build(
            config.web_cookie.clone(),
            if !config.mobile_cookie.is_empty() {
                Some(config.mobile_cookie.clone())
            } else {
                None
            },
        );
        let persister = Persister::build(config.db.clone())?;
        Ok(TaskHandler {
            fetcher,
            persister,
            config,
        })
    }

    pub async fn fetch_all_page(&self) -> Result<()> {
        let fav_total_num = self.fetcher.fetch_fav_total_num().await?;
        info!("there are {fav_total_num} fav posts in total");
        let mut total_posts_sum = 0;
        for page in 1.. {
            let mut posts = self
                .fetcher
                .fetch_posts_meta(self.config.uid.as_str(), page)
                .await?;
            let posts_sum = posts.len();
            total_posts_sum += posts_sum;
            debug!("fetched {} posts in {}th page", posts_sum, page);
            if posts_sum == 0 {
                info!("no more posts in {}th page, finish work", page);
                break;
            }
            for post in posts.iter_mut() {
                self.process_post(post).await?;
            }
            sleep(Duration::from_secs(5)).await;
        }
        info!("fetched {total_posts_sum} posts in total");
        Ok(())
    }

    async fn process_post(&self, post: &mut FetchedPost) -> Result<()> {
        self.process_post_non_rec(post).await?;
        if let Some(mut post) = post.retweeted_status.take() {
            self.process_post_non_rec(post.as_mut()).await?;
        }
        Ok(())
    }

    async fn process_pic(&self, url: &str) -> Result<()> {
        if self.persister.query_img(url).is_err() {
            debug!("query pic failed, fetch: {url}");
            let pic_blob = self.fetcher.fetch_pic(url).await.unwrap();
            self.persister.insert_img(url, &pic_blob).unwrap();
        } else {
            debug!("pic already saved, skip: {url}");
        }
        Ok(())
    }

    async fn process_post_non_rec(&self, post: &mut FetchedPost) -> Result<()> {
        if let Some(user) = &post.user {
            self.persister.insert_user(user)?;
        }
        if post.is_long_text && !post.continue_tag.is_null() {
            let content = self.fetcher.fetch_long_text_content(&post.mblogid).await?;
            post.text_raw = content;
        }
        if let Some(user) = &post.user {
            let avatar_url = strip_url_queries(&user.avatar_hd).unwrap();
            self.process_pic(avatar_url).await?;
        }
        if let Some(pics) = &post.pic_ids {
            for id in pics.iter() {
                let pic_info = post.pic_infos.get(id).unwrap();
                let url = pic_info
                    .pic_resources
                    .get("mw2000")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("url")
                    .unwrap()
                    .as_str()
                    .unwrap();
                let url = strip_url_queries(url).unwrap();
                self.process_pic(url).await?;
            }
        }
        self.persister.insert_post(post)?;
        Ok(())
    }
}

fn strip_url_queries<'a>(url: &'a str) -> Option<&'a str> {
    url.split('?').next()
}
