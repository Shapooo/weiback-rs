use anyhow::Result;
// use log::{debug, info, trace};
use log::{info, trace};
use rusqlite::{named_params, params, Connection};
use std::path::PathBuf;

// use serde_json::Value;

use crate::meta_data::{Post, PostUser};

#[derive(Debug)]
pub struct Persister {
    conn: Connection,
}

impl Persister {
    pub fn build(db: PathBuf) -> Result<Self> {
        if db.is_file() {
            info!("db file {} exists", db.display());
        } else {
            info!("db file {} not exists, create it", db.display());
        }
        let conn = Connection::open(db)?;
        conn.execute_batch(
            "CREATE TABLE
  IF NOT EXISTS fav_post(
    id INTEGER PRIMARY KEY NOT NULL UNIQUE,
    visible json,
    created_at VARCHAR,
    mblogid VARCHAR,
    uid INT,
    can_edit BOOLEAN,
    text_raw text,
    text text,
    annotations json,
    source VARCHAR,
    favorited BOOLEAN,
    rid VARCHAR,
    cardid VARCHAR,
    pic_ids VARCHAR,
    pic_infos json,
    geo json,
    pic_num INT,
    pic_focus_point json,
    is_paid BOOLEAN,
    pic_bg_new VARCHAR,
    topic_struct json,
    page_info json,
    deleted BOOLEAN,
    mark VARCHAR,
    tag_struct json,
    title json,
    mblog_vip_type INT,
    number_display_strategy json,
    reposts_count INT,
    comments_count INT,
    attitudes_count INT,
    attitudes_status INT,
    continue_tag json,
    isLongText BOOLEAN,
    mlevel INT,
    content_auth INT,
    is_show_bulletin INT,
    comment_manage_info json,
    repost_type INT,
    share_repost_type INT,
    url_struct json,
    retweeted_status INT,
    edit_count INT,
    textLength INT,
    mblogtype INT,
    showFeedRepost BOOLEAN,
    showFeedComment BOOLEAN,
    pictureViewerSign BOOLEAN,
    showPictureViewer BOOLEAN,
    region_name VARCHAR,
    mix_media_info json
  );

CREATE TABLE
  IF NOT EXISTS user(
    id INT PRIMARY KEY NOT NULL,
    pc_new INT,
    screen_name VARCHAR,
    profile_url VARCHAR,
    profile_image_url VARCHAR,
    avatar_hd VARCHAR,
    avatar_large VARCHAR,
    verified BOOLEAN,
    verified_type INT,
    domain VARCHAR,
    weihao VARCHAR,
    verified_type_ext INT,
    follow_me BOOLEAN,
    following BOOLEAN,
    mbrank INT,
    mbtype INT,
    v_plus INT,
    planet_video BOOLEAN,
    icon_list json
  );

  CREATE TABLE
  IF NOT EXISTS pic_blob(
    url VARCHAR PRIMARY KEY NOT NULL,
    pic_blob BLOB
  );
  ",
        )?;
        Ok(Persister { conn })
    }

    pub fn insert_post(&self, post: &Post) -> Result<()> {
        self.conn.execute_batch(&post.to_sql())?;
        trace!("insert {}", post.to_sql());
        Ok(())
    }

    pub fn insert_img(&self, url: &str, img: &[u8]) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("INSERT OR IGNORE INTO pic_blob (url, pic_blob) VALUES (:url, :blob)")?;
        stmt.execute(named_params! {":blob":img, ":url": url})?;
        Ok(())
    }

    pub fn insert_user(&self, user: &PostUser) -> Result<()> {
        self.conn.execute_batch(&user.to_sql())?;
        trace!("insert {}", user.to_sql());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::meta_data::Posts;

    use super::*;
    use serde_json;
    #[test]
    fn post_build() {
        Persister::build("post_build.db".into()).unwrap();
        std::fs::remove_file("post_build.db").unwrap();
    }

    #[test]
    fn insert_post() {
        let pster = Persister::build("insert_post.db".into()).unwrap();
        let txt = include_str!("../res/one.json");
        let post: Post = serde_json::from_str(txt).unwrap();
        pster.insert_post(&post).unwrap();
        std::fs::remove_file("insert_post.db").unwrap();
    }

    #[test]
    fn insert_posts() {
        let pster = Persister::build("insert_posts.db".into()).unwrap();
        let txt = include_str!("../res/full.json");
        let posts: Posts = serde_json::from_str(txt).unwrap();
        posts
            .data
            .iter()
            .for_each(|p| pster.insert_post(p).unwrap());
        std::fs::remove_file("insert_posts.db").unwrap();
    }

    #[test]
    fn insert_users() {
        let pster = Persister::build("insert_user.db".into()).unwrap();
        let txt = include_str!("../res/full.json");
        let posts: Posts = serde_json::from_str(txt).unwrap();
        posts.data.iter().for_each(|p| {
            if p.user.is_some() {
                pster.insert_user(&p.user.clone().unwrap()).unwrap()
            }
        });
        std::fs::remove_file("insert_user.db").unwrap();
    }

    #[test]
    fn insert_img() {
        let img = include_bytes!("../res/example.jpg");
        let p = Persister::build("insert_img.db".into()).unwrap();
        p.insert_img("https://test_url/example.jpg", img).unwrap();
        std::fs::remove_file("insert_img.db").unwrap();
    }
}
