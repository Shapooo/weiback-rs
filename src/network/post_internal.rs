use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::{HashMap, HashSet},
    ops::DerefMut,
    path::Path,
};

use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, FixedOffset};
use lazy_static::lazy_static;
use log::{debug, trace};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};
use sqlx::{Executor, FromRow, Sqlite};

use super::user_client::UserInternal;
use crate::app::service::{search_args::SearchArgs, emoticon::emoticon_get};

const FAVORITES_ALL_FAV_API: &str = "https://weibo.com/ajax/favorites/all_fav";
const MOBILE_POST_API: &str = "https://m.weibo.cn/statuses/show?id=";
const POST_SEARCH_API: &str = "https://weibo.com/ajax/statuses/searchProfile";
const DESTROY_FAVORITES: &str = "https://weibo.com/ajax/statuses/destoryFavorites";

lazy_static! {
    static ref NEWLINE_EXPR: Regex = Regex::new("\\n").unwrap();
    static ref URL_EXPR: Regex =
        Regex::new("(http|https)://[a-zA-Z0-9$%&~_#/.\\-:=,?]{5,280}").unwrap();
    static ref AT_EXPR: Regex = Regex::new(r"@[\u4e00-\u9fa5|\uE7C7-\uE7F3|\w_\-·]+").unwrap();
    static ref EMOJI_EXPR: Regex = Regex::new(r"(\[.*?\])").unwrap();
    static ref EMAIL_EXPR: Regex =
        Regex::new(r"[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}").unwrap();
    static ref TOPIC_EXPR: Regex = Regex::new(r#"#([^#]+)#"#).unwrap();
}

const IMG_TYPES: &[&[&str; 6]; 3] = &[
    &[
        "thunmnail",
        "bmiddle",
        "large",
        "original",
        "largest",
        "mw2000",
    ],
    &[
        "large",
        "original",
        "bmiddle",
        "largest",
        "thumbnail",
        "mw2000",
    ],
    &[
        "mw2000",
        "largest",
        "original",
        "large",
        "bmiddle",
        "thumbnail",
    ],
];

#[derive(Deserialize, Serialize, Debug, Clone, FromRow, PartialEq)]
struct PostInternal {
    pub id: i64,
    pub mblogid: String,
    pub text_raw: String,
    pub source: String,
    pub region_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_deleted")]
    pub deleted: bool,
    pub pic_ids: Option<Value>,
    pub pic_num: Option<i64>,
    pub url_struct: Option<Value>,
    pub topic_struct: Option<Value>,
    pub tag_struct: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_vec_value")]
    pub tags: Option<Value>,
    #[sqlx(rename = "customIcons")]
    #[serde(
        default,
        rename = "customIcons",
        deserialize_with = "deserialize_vec_value"
    )]
    pub custom_icons: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub visible: Value,
    pub text: String,
    #[sqlx(default)]
    #[serde(default)]
    pub attitudes_status: i64,
    #[sqlx(default, rename = "showFeedRepost")]
    #[serde(default, rename = "showFeedRepost")]
    pub show_feed_repost: bool,
    #[sqlx(default, rename = "showFeedComment")]
    #[serde(default, rename = "showFeedComment")]
    pub show_feed_comment: bool,
    #[sqlx(default, rename = "pictureViewerSign")]
    #[serde(default, rename = "pictureViewerSign")]
    pub picture_viewer_sign: bool,
    #[sqlx(default, rename = "showPictureViewer")]
    #[serde(default, rename = "showPictureViewer")]
    pub show_picture_viewer: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub favorited: bool,
    pub can_edit: Option<bool>,
    pub is_paid: Option<bool>,
    pub share_repost_type: Option<i64>,
    pub rid: Option<String>,
    pub pic_infos: Option<Value>,
    pub cardid: Option<String>,
    pub pic_bg_new: Option<String>,
    pub mark: Option<String>,
    pub mblog_vip_type: Option<i64>,
    pub reposts_count: Option<i64>,
    pub comments_count: Option<i64>,
    pub attitudes_count: Option<i64>,
    pub mlevel: Option<i64>,
    pub complaint: Option<Value>,
    pub content_auth: Option<i64>,
    pub is_show_bulletin: Option<i64>,
    pub repost_type: Option<i64>,
    pub edit_count: Option<i64>,
    pub mblogtype: Option<i64>,
    #[sqlx(rename = "textLength")]
    #[serde(rename = "textLength")]
    pub text_length: Option<i64>,
    #[sqlx(default, rename = "isLongText")]
    #[serde(default, rename = "isLongText")]
    pub is_long_text: bool,
    #[sqlx(rename = "rcList")]
    #[serde(default, rename = "rcList", deserialize_with = "deserialize_vec_value")]
    pub rc_list: Option<Value>,
    pub annotations: Option<Value>,
    pub geo: Option<Value>,
    pub pic_focus_point: Option<Value>,
    pub page_info: Option<Value>,
    pub title: Option<Value>,
    pub continue_tag: Option<Value>,
    pub comment_manage_info: Option<Value>,
    #[sqlx(default)]
    #[serde(skip)]
    pub client_only: bool,
    #[sqlx(default)]
    #[serde(skip)]
    pub unfavorited: bool,
    pub created_at: String,
    #[serde(skip)]
    pub created_at_timestamp: i64,
    #[serde(skip)]
    pub created_at_tz: String,
    #[sqlx(skip)]
    pub retweeted_status: Option<Box<PostInternal>>,
    #[sqlx(skip)]
    #[serde(default, deserialize_with = "deserialize_user")]
    pub user: Option<UserInternal>,
}

fn deserialize_vec_value<'de, D>(deserializer: D) -> std::result::Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;

    if let Some(Value::Array(arr)) = &value {
        if arr.is_empty() {
            return Ok(None);
        }
    }
    Ok(value)
}

fn deserialize_user<'de, D>(deserializer: D) -> std::result::Result<Option<UserInternal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    if value.is_null() || value["id"].is_null() {
        Ok(None)
    } else {
        let user = match from_value(value) {
            Ok(user) => user,
            Err(e) => return Err(serde::de::Error::custom(e)),
        };
        Ok(Some(user))
    }
}

fn deserialize_deleted<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;
    Ok(str == "1")
}

impl TryFrom<Value> for PostInternal {
    type Error = Error;
    fn try_from(mut json: Value) -> Result<Self> {
        // struct of post is different on mobile and pc web,
        // convert to pc format in advance
        if json["id"].is_string() {
            Self::convert_mobile2pc_post(&mut json)?;
        }
        let created_at = json["created_at"]
            .as_str()
            .map(parse_created_at)
            .ok_or(anyhow!("invalid created_at field"))??;
        let mut post: PostInternal = from_value(json)?;
        post.uid = post.user.as_ref().map(|user| user.id);
        post.created_at_timestamp = created_at.timestamp();
        post.created_at_tz = created_at.timezone().to_string();
        post.created_at = created_at.to_string();
        post.client_only = post.is_client_only();
        if let Some(mut retweeted_status) = post.retweeted_status.take() {
            post.retweeted_id = Some(retweeted_status.id);
            retweeted_status.uid = retweeted_status.user.as_ref().map(|user| user.id);
            post.retweeted_status = Some(retweeted_status);
        }

        if let Some(mut retweet) = post.retweeted_status.take() {
            retweet.page_info = post.page_info.take();
            post.retweeted_status = Some(retweet);
        }
        Ok(post)
    }
}

impl TryInto<Value> for PostInternal {
    type Error = Error;
    fn try_into(self) -> Result<Value> {
        Ok(to_value(self)?)
    }
}

impl PostInternal {
    fn into_tera_context_val(
        mut self,
        pictures: &mut HashSet<Picture>,
        resource_dir: &Path,
        image_definition: u8,
    ) -> Result<Value> {
        let pic_urls = self
            .extract_pic_urls(image_definition)
            .into_iter()
            .map(|url| Picture::in_post(url, self.id))
            .collect::<Vec<_>>();
        let emoji_urls = self.extract_emoji_urls().into_iter().map(Picture::emoji);
        // pic_locs is to insert into post json
        let pic_locs: Vec<_> = pic_urls
            .iter()
            .map(|pic| resource_dir.join(pic.get_file_name()))
            .collect();

        pic_urls.into_iter().for_each(|pic| {
            pictures.insert(pic);
        });

        emoji_urls.into_iter().for_each(|pic| {
            pictures.insert(pic);
        });

        let new_text = self.trans_text(resource_dir)?;
        trace!("conv {} to {}", self.text_raw, new_text);
        self.text_raw = new_text;
        let avatar_file = self.user.as_ref().map(|user| {
            let avatar = user.get_avatar_pic(image_definition);
            let avatar_file = resource_dir.join(avatar.get_file_name());
            pictures.insert(avatar);
            avatar_file
        });
        let retweeter_avatar_file = self
            .retweeted_status
            .as_ref()
            .and_then(|retweeted| retweeted.user.as_ref())
            .map(|user| {
                let avatar = user.get_avatar_pic(image_definition);
                let avatar_file = resource_dir.join(avatar.get_file_name());
                pictures.insert(avatar);
                avatar_file
            });

        let mut post = to_value(self)?;
        if !pic_locs.is_empty() {
            post["pics"] = to_value(pic_locs).unwrap();
        }
        post["poster_avatar"] = to_value(avatar_file).unwrap();
        if post["retweeted_status"].is_object() && post["retweeted_status"]["user"].is_object() {
            post["retweeted_status"]["poster_avatar"] = to_value(retweeter_avatar_file)?
        }

        Ok(post)
    }
}

impl PostInternal {
    pub fn get_unfavorite_url() -> String {
        DESTROY_FAVORITES.into()
    }

    pub fn get_posts_download_url(uid: i64, page: u32, search_args: &SearchArgs) -> String {
        let mut url = format!("{}?uid={}&page={}", POST_SEARCH_API, uid, page);
        url = search_args.attach_args(url);
        url
    }

    pub fn get_favorite_download_url(uid: i64, page: u32) -> String {
        format!("{FAVORITES_ALL_FAV_API}?uid={uid}&page={page}")
    }

    pub fn get_mobile_download_url(mblogid: &str) -> String {
        // let mobile_client = &self.mobile_client;
        format!("{}{}", MOBILE_POST_API, mblogid)
    }

    // fn with_process_long_text(mut post: Post, long_text: String) -> Post {
    //     if post.is_long_text {
    //         post.text_raw = long_text;
    //     }
    //     post
    // }

    // async fn with_process_client_only(&self, mut post: Post) -> Result<Post> {
    //     if post.client_only {
    //         post = self.get_mobile_post(&post.mblogid).await?;
    //     }
    //     Ok(post)
    // }

    // async fn posts_process(&self, posts: Vec<Value>) -> Result<Vec<Post>> {
    //     let posts = posts
    //         .into_iter()
    //         .map(|post| post.try_into())
    //         .collect::<Result<Vec<Post>>>()?;
    //     debug!("get raw {} posts", posts.len());
    //     let posts = join_all(posts.into_iter().map(|post| async {
    //         let post = self.with_process_client_only(post).await?;
    //         // self.with_process_long_text(fetcher)
    //         // .await
    //         anyhow::Ok(post)
    //     }))
    //     .await
    //     .into_iter()
    //     .filter_map(|post| match post {
    //         // network errors usually recoverable, so just ignore it
    //         // TODO: collect failed post and retry
    //         Ok(post) => Some(post),
    //         Err(e) => {
    //             error!("process post failed: {}", e);
    //             None
    //         }
    //     })
    //     .collect::<Vec<_>>();
    //     Ok(posts)
    // }

    fn is_client_only(&self) -> bool {
        self.user.is_none() && self.text_raw.starts_with("该内容请至手机客户端查看")
    }

    fn convert_mobile2pc_post(post: &mut Value) -> Result<()> {
        Self::convert_mobile2pc_post_non_rec(post)?;
        if post["retweeted_status"].is_object() {
            Self::convert_mobile2pc_post_non_rec(&mut post["retweeted_status"])?;
        }
        Ok(())
    }

    fn convert_mobile2pc_post_non_rec(post: &mut Value) -> Result<()> {
        let id = post["id"]
            .as_str()
            .ok_or(anyhow!("mobile post id should be str: {}", post))?;
        let id = match id.parse::<i64>() {
            Ok(id) => id,
            Err(e) => return Err(anyhow!("failed to parse mobile post id {id}: {e}")),
        };
        post["id"] = Value::Number(serde_json::Number::from(id));
        post["mblogid"] = post["bid"].take();
        post["text_raw"] = post["text"].clone();
        post["favorited"] = Value::Bool(true);
        if let Value::Array(arr) = post["url_objects"].take() {
            post["url_struct"] = Value::Array(
                arr.into_iter()
                    .map(|mut obj| {
                        let mut url_struct: HashMap<String, Value> = HashMap::new();
                        url_struct.insert("url_title".into(), obj["url_ori"].clone());
                        url_struct.insert("ori_url".into(), obj["url_ori"].take());
                        url_struct.insert("url_short".into(), obj["info"]["url_short"].take());
                        url_struct.insert("url_long".into(), obj["info"]["url_long"].take());
                        url_struct.insert("url_type".into(), obj["info"]["type"].take());
                        url_struct.insert("result".into(), obj["info"]["result"].take());
                        Ok(serde_json::to_value(url_struct)?)
                    })
                    .collect::<Result<Vec<Value>>>()?,
            );
        }
        if let Value::Array(pics) = post["pics"].take() {
            post["pic_ids"] = serde_json::to_value(
                pics.iter()
                    .map(|pic| {
                        pic["pid"]
                            .as_str()
                            .ok_or(anyhow!("pid of mobile post pic should be a str: {}", pic))
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
            .unwrap();
            post["pic_infos"] = serde_json::to_value(
                pics.into_iter()
                    .map(|mut pic| {
                        let id = pic["pid"]
                            .as_str()
                            .ok_or(anyhow!("pid of mobile post pic should be a str: {}", pic))?
                            .to_owned();
                        let mut v: HashMap<String, Value> = HashMap::new();
                        v.insert("pic_id".into(), pic["pid"].take());
                        v.insert("type".into(), "pic".into());
                        v.insert("large".into(), pic["large"].take());
                        v.insert(
                            "bmiddle".into(),
                            serde_json::json!({"url":pic["url"].take()}),
                        );
                        Ok((id, serde_json::to_value(v).unwrap()))
                    })
                    .collect::<Result<HashMap<String, Value>>>()?,
            )
            .unwrap();
        }
        Ok(())
    }

    fn extract_emojis(&self) -> Vec<&str> {
        EMOJI_EXPR
            .find_iter(&self.text_raw)
            .map(|e| e.as_str())
            .collect()
    }

    fn extract_emoji_urls(&self) -> Vec<&str> {
        self.extract_emojis()
            .into_iter()
            .filter_map(|emoji| emoticon_get(emoji))
            .collect()
    }

    fn extract_pic_urls(&self, image_definition: u8) -> Vec<&str> {
        let mut pic_vec = if let (Some(pic_ids), Some(pic_infos)) =
            (self.pic_ids.as_ref(), self.pic_infos.as_ref())
        {
            PostInternal::pic_ids_to_urls(pic_ids, pic_infos, image_definition)
        } else {
            Default::default()
        };
        if let Some(retweeted_post) = &self.retweeted_status {
            let mut retweeted_pic_vec = retweeted_post.extract_pic_urls(image_definition);
            pic_vec.append(retweeted_pic_vec.as_mut());
        }
        pic_vec
    }

    fn pic_ids_to_urls<'a>(
        pic_ids: &'a Value,
        pic_infos: &'a Value,
        image_definition: u8,
    ) -> Vec<&'a str> {
        if let Value::Array(pic_ids) = pic_ids {
            if !pic_ids.is_empty() {
                let pic_urls: Vec<_> = pic_ids
                    .iter()
                    .filter_map(|id| id.as_str())
                    .filter_map(|id| PostInternal::select_pic_url(&pic_infos[id], image_definition))
                    .collect();
                pic_urls
            } else {
                Default::default()
            }
        } else {
            Default::default()
        }
    }

    fn select_pic_url(pic_info: &Value, image_definition: u8) -> Option<&str> {
        if pic_info.is_null() {
            return None;
        }
        IMG_TYPES[image_definition as usize]
            .iter()
            .find(|t| pic_info[t].is_object())
            .and_then(|t| pic_info[t]["url"].as_str())
    }

    fn trans_text(&self, pic_folder: &Path) -> Result<String> {
        let emails_suffixes = EMAIL_EXPR
            .find_iter(&self.text_raw)
            .filter_map(|m| AT_EXPR.find(m.as_str()).map(|m| m.as_str()))
            .collect::<HashSet<_>>();
        let text = NEWLINE_EXPR.replace_all(&self.text_raw, "<br />");
        let text = {
            let res = URL_EXPR
                .find_iter(&text)
                .fold((Borrowed(""), 0), |(acc, i), m| {
                    (
                        acc + &text[i..m.start()] + self.trans_url(&text[m.start()..m.end()]),
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
                            + PostInternal::trans_user(&text[m.start()..m.end()]),
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
                        acc + &text[i..m.start()]
                            + PostInternal::trans_topic(&text[m.start()..m.end()]),
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
                            + Self::trans_emoji(&text[m.start()..m.end()], pic_folder),
                        m.end(),
                    )
                });
            res.0 + Borrowed(&text[res.1..])
        };

        Ok(text.to_string())
    }

    fn trans_emoji<'a>(s: &'a str, pic_folder: &'a Path) -> Cow<'a, str> {
        if let Some(url) = emoticon_get(s) {
            let pic = Picture::emoji(url);
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

    fn trans_url<'a>(&self, s: &'a str) -> Cow<'a, str> {
        let mut url_title = Borrowed("网页链接");
        let mut url = Borrowed(s);
        if let Some(Value::Array(url_objs)) = self.url_struct.as_ref() {
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

    pub async fn generate_html<E>(
        posts: Vec<PostInternal>,
        html_name: &str,
        image_definition: u8,
        mut executor: E,
        fetcher: &WebFetcher,
    ) -> Result<HTMLPage>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        debug!("generate html from {} posts", posts.len());
        let mut pic_to_fetch = HashSet::new();
        let posts = posts
            .into_iter()
            .map(|post| {
                post.into_tera_context_val(
                    &mut pic_to_fetch,
                    Path::new((Borrowed(html_name) + "_files").as_ref()),
                    image_definition,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let inner_html = HTMLGenerator::generate_posts(posts)?;
        let html = HTMLGenerator::generate_page(&inner_html)?;
        let mut pics = Vec::new();
        for pic in pic_to_fetch {
            if let Some(blob) = pic.get_blob(&mut *executor, fetcher).await? {
                pics.push(HTMLPicture {
                    name: pic.get_file_name().into(),
                    blob,
                });
            }
        }
        Ok(HTMLPage { html, pics })
    }
}

pub fn parse_created_at(created_at: &str) -> Result<DateTime<FixedOffset>> {
    match DateTime::parse_from_str(created_at, "%a %b %d %T %z %Y") {
        Ok(dt) => Ok(dt),
        Err(e) => Err(anyhow!("{e}")),
    }
}

#[cfg(test)]
mod post_test {
    use super::*;
    use flate2::read::GzDecoder;
    use std::io::prelude::*;

    async fn create_db() -> anyhow::Result<sqlx::SqlitePool> {
        Ok(sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await?)
    }

    fn load_test_case() -> anyhow::Result<String> {
        let gz = include_bytes!("../../res/full.json.gz");
        let mut de = GzDecoder::new(gz.as_ref());
        let mut txt = String::new();
        de.read_to_string(&mut txt).unwrap();
        Ok(txt)
    }

    #[tokio::test]
    async fn create_table() {
        let db = create_db().await.unwrap();
        let mut conn = db.acquire().await.unwrap();
        PostInternal::create_table(conn.as_mut()).await.unwrap();
    }

    #[test]
    fn deserialize_posts() {
        let test_case = load_test_case().unwrap();
        let test_case_val = serde_json::from_str::<Value>(&test_case).unwrap();
        let test_case_val_vec = serde_json::from_str::<Vec<Value>>(&test_case).unwrap();

        let _: Vec<PostInternal> = serde_json::from_str(&test_case).unwrap();
        let _: Vec<PostInternal> = serde_json::from_value(test_case_val).unwrap();
        let _: Vec<PostInternal> = test_case_val_vec
            .into_iter()
            .map(|v| serde_json::from_value(v).unwrap())
            .collect();
    }

    #[test]
    fn posts_try_from_value() {
        let test_case = load_test_case().unwrap();
        let posts: Vec<Value> = serde_json::from_str(&test_case).unwrap();
        for post in posts {
            let postb = post.clone();
            let _: PostInternal = post
                .try_into()
                .map_err(|e| {
                    format!(
                        "failed to convert post {post:?} to Post: {e}",
                        post = postb,
                        e = e
                    )
                })
                .unwrap();
        }
    }

    #[tokio::test]
    async fn insert() {
        let ref db = create_db().await.unwrap();
        let mut trans = db.begin().await.unwrap();
        PostInternal::create_table(trans.as_mut()).await.unwrap();
        UserInternal::create_table(trans.as_mut()).await.unwrap();

        let test_case = serde_json::from_str::<Vec<Value>>(&load_test_case().unwrap())
            .unwrap()
            .into_iter()
            .map(|v| v.try_into().unwrap())
            .collect::<Vec<PostInternal>>();
        for post in test_case {
            post.insert(trans.as_mut()).await.unwrap();
        }
        trans.commit().await.unwrap();
    }

    #[tokio::test]
    async fn query() {
        let ref db = create_db().await.unwrap();
        let mut trans = db.begin().await.unwrap();
        PostInternal::create_table(trans.as_mut()).await.unwrap();
        UserInternal::create_table(trans.as_mut()).await.unwrap();

        let test_case = serde_json::from_str::<Vec<Value>>(&load_test_case().unwrap()).unwrap();
        let mut posts: HashMap<i64, PostInternal> = HashMap::new();
        test_case.into_iter().for_each(|v| {
            let post: PostInternal = v.try_into().unwrap();
            if posts.contains_key(&post.id) {
                return;
            }
            if let Some(retweeted) = &post.retweeted_status {
                if posts.contains_key(&retweeted.id) {
                    return;
                }
                posts.insert(retweeted.id, retweeted.as_ref().clone());
            }
            posts.insert(post.id, post);
        });
        for post in posts.values() {
            post.insert(trans.as_mut()).await.unwrap();
        }

        for &id in posts.keys() {
            let mut origin_post = posts.get(&id).unwrap().clone();
            let mut post = PostInternal::query(id, trans.as_mut())
                .await
                .unwrap()
                .unwrap();
            origin_post.user = None;
            origin_post.retweeted_status.as_mut().map(|p| p.user = None);
            post.user = None;
            post.retweeted_status.as_mut().map(|p| p.user = None);
            assert_eq!(origin_post, post);
        }
    }

    #[test]
    fn parse_datetime() {
        parse_created_at("Mon May 29 19:29:32 +0800 2023").unwrap();
        parse_created_at("Mon May 29 19:45:00 +0800 2023").unwrap();
        parse_created_at("Tue May 30 04:07:49 +0800 2023").unwrap();
    }
}
