use crate::{
    emoticon::emoticon_get,
    error::{Error, Result},
    exporter::{HTMLPage, HTMLPicture},
    html_generator::HTMLGenerator,
    long_text::LongText,
    picture::Picture,
    user::User,
    web_fetcher::WebFetcher,
};

use std::collections::{HashMap, HashSet};
use std::{
    borrow::Cow::{self, Borrowed, Owned},
    path::Path,
};

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use futures::future::join_all;
use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};
use sqlx::{FromRow, Sqlite, SqlitePool};

const FAVORITES_ALL_FAV_API: &str = "https://weibo.com/ajax/favorites/all_fav";
const MOBILE_POST_API: &str = "https://m.weibo.cn/statuses/show?id=";
const STATUSES_MY_MICRO_BLOG_API: &str = "https://weibo.com/ajax/statuses/mymblog";
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
pub struct Post {
    pub id: i64,
    pub mblogid: String,
    pub text_raw: String,
    pub source: String,
    pub region_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_deleted")]
    pub deleted: bool,
    pub uid: Option<i64>,
    pub pic_ids: Option<Value>,
    pub pic_num: Option<i64>,
    #[serde(skip)]
    pub retweeted_status: Option<i64>,
    pub url_struct: Option<Value>,
    pub topic_struct: Option<Value>,
    pub tag_struct: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub visible: Value,
    pub text: String,
    #[sqlx(default)]
    pub attitudes_status: i64,
    #[sqlx(default, rename = "showFeedRepost")]
    #[serde(rename = "showFeedRepost")]
    pub show_feed_repost: bool,
    #[sqlx(default, rename = "showFeedComment")]
    #[serde(rename = "showFeedComment")]
    pub show_feed_comment: bool,
    #[sqlx(default, rename = "pictureViewerSign")]
    #[serde(rename = "pictureViewerSign")]
    pub picture_viewer_sign: bool,
    #[serde(rename = "showPictureViewer")]
    #[sqlx(default, rename = "showPictureViewer")]
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
    pub content_auth: Option<i64>,
    pub is_show_bulletin: Option<i64>,
    pub repost_type: Option<i64>,
    pub edit_count: Option<i64>,
    pub mblogtype: Option<i64>,
    #[sqlx(rename = "textLength")]
    #[serde(rename = "textLength")]
    pub text_length: Option<i64>,
    #[serde(default, rename = "isLongText")]
    #[sqlx(default, rename = "isLongText")]
    pub is_long_text: bool,
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
    #[sqlx(skip)]
    pub created_at: String,
    #[sqlx(rename = "created_at")]
    #[serde(skip)]
    pub created_at_timestamp: i64,
    #[serde(skip)]
    pub created_at_tz: String,
    #[sqlx(skip)]
    #[serde(rename = "retweeted_status")]
    pub retweeted_post: Option<Box<Post>>,
    #[sqlx(skip)]
    #[serde(deserialize_with = "deserialize_user")]
    pub user: Option<User>,
}

fn deserialize_user<'de, D>(deserializer: D) -> std::result::Result<Option<User>, D::Error>
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

impl TryFrom<Value> for Post {
    type Error = Error;
    fn try_from(mut json: Value) -> Result<Self> {
        // struct of post is different on mobile and pc web,
        // convert to pc format in advance
        if json["id"].is_string() {
            Self::convert_mobile2pc_post(&mut json)?;
        }
        let created_at = if let Value::String(created_at) = &json["created_at"] {
            parse_created_at(created_at)?
        } else {
            return Err(Error::MalFormat("invalid created_at field".into()));
        };
        let mut post: Post = from_value(json)?;
        post.uid = post.user.as_ref().map(|user| user.id);
        post.created_at_timestamp = created_at.timestamp();
        post.created_at_tz = created_at.timezone().to_string();
        post.created_at = created_at.to_string();
        post.retweeted_status = post.retweeted_post.as_ref().map(|post| post.id);
        post.client_only = post.is_client_only();

        if let Some(mut retweet) = post.retweeted_post.take() {
            retweet.page_info = post.page_info.take();
            if let Some(Value::Array(url_struct)) = post.url_struct.take() {
                let url_struct = url_struct
                    .into_iter()
                    .filter_map(|st| {
                        let short_url = st["short_url"].as_str().map(|s| s.to_owned());
                        short_url.map(|url| (url, st))
                    })
                    .collect::<HashMap<String, Value>>();
                let ret_url_struct = extract_urls(&retweet.text_raw)
                    .into_iter()
                    .filter_map(|url| url_struct.get(url))
                    .cloned()
                    .collect::<Vec<_>>();
                let ret_len = ret_url_struct.len();
                if !ret_url_struct.is_empty() {
                    retweet.url_struct = Some(Value::Array(ret_url_struct));
                }
                let post_url_struct = extract_urls(&post.text_raw)
                    .into_iter()
                    .filter_map(|url| url_struct.get(url))
                    .cloned()
                    .collect::<Vec<_>>();
                let post_len = post_url_struct.len();
                if !post_url_struct.is_empty() {
                    post.url_struct = Some(Value::Array(post_url_struct));
                }
                if post_len + ret_len < url_struct.len() {
                    warn!(
                        "{} url_struct is not used",
                        url_struct.len() - post_len - ret_len
                    );
                }
            }
            // TODO: handle tag_struct
            post.retweeted_post = Some(retweet);
        }
        Ok(post)
    }
}

impl TryInto<Value> for Post {
    type Error = Error;
    fn try_into(self) -> Result<Value> {
        let mut value = serde_json::to_value(self)?;
        let timestamp = value["created_at_timestamp"].take();
        let tz = value["created_at_tz"].take();
        // Convert created_at field to datetime string
        if let (Some(timestamp), Some(tz)) = (timestamp.as_i64(), tz.as_str()) {
            value["created_at"] = to_value(
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    // TODO: remove unwrap, return error
                    NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap(),
                    tz.parse().unwrap(),
                )
                .to_string(),
            )
            .unwrap();
        }
        Ok(value)
    }
}

impl Post {
    fn to_tera_context_val(
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

        let mut post = to_value(self)?;
        if !pic_locs.is_empty() {
            post["pics"] = to_value(pic_locs).unwrap();
        }
        post["poster_avatar"] = to_value(avatar_file).unwrap();

        Ok(post)
    }
}

impl Post {
    pub async fn create_table(db: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS posts ( \
             id INTEGER PRIMARY KEY, \
             mblogid TEXT, \
             text_raw TEXT, \
             source TEXT, \
             region_name TEXT, \
             deleted INTEGER, \
             uid INTEGER, \
             pic_ids TEXT, \
             pic_num INTEGER, \
             retweeted_status INTEGER, \
             url_struct TEXT, \
             topic_struct TEXT, \
             tag_struct TEXT, \
             number_display_strategy TEXT, \
             mix_media_info TEXT, \
             visible TEXT, \
             text TEXT, \
             attitudes_status INTEGER, \
             showFeedRepost INTEGER, \
             showFeedComment INTEGER, \
             pictureViewerSign INTEGER, \
             showPictureViewer INTEGER, \
             favorited INTEGER, \
             can_edit INTEGER, \
             is_paid INTEGER, \
             share_repost_type INTEGER, \
             rid TEXT, \
             pic_infos TEXT, \
             cardid TEXT, \
             pic_bg_new TEXT, \
             mark TEXT, \
             mblog_vip_type INTEGER, \
             reposts_count INTEGER, \
             comments_count INTEGER, \
             attitudes_count INTEGER, \
             mlevel INTEGER, \
             content_auth INTEGER, \
             is_show_bulletin INTEGER, \
             repost_type INTEGER, \
             edit_count INTEGER, \
             mblogtype INTEGER, \
             textLength INTEGER, \
             isLongText INTEGER, \
             annotations TEXT, \
             geo TEXT, \
             pic_focus_point TEXT, \
             page_info TEXT, \
             title TEXT, \
             continue_tag TEXT, \
             comment_manage_info TEXT, \
             client_only INTEGER, \
             unfavorited INTEGER, \
             created_at INTEGER, \
             created_at_tz TEXT \
             )",
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn insert(&self, db: &SqlitePool) -> Result<()> {
        debug!("insert post: {}", self.id);
        trace!("insert post: {:?}", self);
        self._insert(db).await?;
        if let Some(retweeted_post) = &self.retweeted_post {
            retweeted_post._insert(db).await?;
        }
        Ok(())
    }

    async fn _insert(&self, db: &SqlitePool) -> Result<()> {
        if let Some(user) = &self.user {
            user.insert(db).await?;
        }
        sqlx::query(
            "INSERT OR IGNORE INTO posts \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(self.id)
        .bind(&self.mblogid)
        .bind(&self.text_raw)
        .bind(&self.source)
        .bind(&self.region_name)
        .bind(self.deleted)
        .bind(self.uid)
        .bind(&self.pic_ids)
        .bind(self.pic_num)
        .bind(self.retweeted_status)
        .bind(&self.url_struct)
        .bind(&self.topic_struct)
        .bind(&self.tag_struct)
        .bind(&self.number_display_strategy)
        .bind(&self.mix_media_info)
        .bind(&self.visible)
        .bind(&self.text)
        .bind(self.attitudes_status)
        .bind(self.show_feed_repost)
        .bind(self.show_feed_comment)
        .bind(self.picture_viewer_sign)
        .bind(self.show_picture_viewer)
        .bind(self.favorited)
        .bind(self.can_edit)
        .bind(self.is_paid)
        .bind(self.share_repost_type)
        .bind(&self.rid)
        .bind(&self.pic_infos)
        .bind(&self.cardid)
        .bind(&self.pic_bg_new)
        .bind(&self.mark)
        .bind(self.mblog_vip_type)
        .bind(self.reposts_count)
        .bind(self.comments_count)
        .bind(self.attitudes_count)
        .bind(self.mlevel)
        .bind(self.content_auth)
        .bind(self.is_show_bulletin)
        .bind(self.repost_type)
        .bind(self.edit_count)
        .bind(self.mblogtype)
        .bind(self.text_length)
        .bind(self.is_long_text)
        .bind(&self.annotations)
        .bind(&self.geo)
        .bind(&self.pic_focus_point)
        .bind(&self.page_info)
        .bind(&self.title)
        .bind(&self.continue_tag)
        .bind(&self.comment_manage_info)
        .bind(self.client_only)
        .bind(self.unfavorited)
        .bind(self.created_at_timestamp)
        .bind(&self.created_at_tz)
        .execute(db)
        .await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn query(id: i64, db: &SqlitePool) -> Result<Option<Post>> {
        debug!("query post, id: {id}");
        if let Some(mut post) = Post::_query(id, db).await? {
            if let Some(retweeted_id) = post.retweeted_status {
                post.retweeted_post = Post::_query(retweeted_id, db).await?.map(Box::new);
            }
            Ok(Some(post))
        } else {
            Ok(None)
        }
    }

    async fn _query(id: i64, db: &SqlitePool) -> Result<Option<Post>> {
        if let Some(post) = sqlx::query_as::<sqlx::Sqlite, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?
        {
            if let Some(uid) = post.uid {
                User::query(uid, db).await?;
            }
            return Ok(Some(post));
        }
        Ok(None)
    }

    pub async fn query_posts(
        limit: u32,
        offset: u32,
        reverse: bool,
        db: &SqlitePool,
    ) -> Result<Vec<Post>> {
        debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
        let sql_expr = if reverse {
            "SELECT * FROM posts WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
        } else {
            "SELECT * FROM posts WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
        };
        let posts = sqlx::query_as::<sqlx::Sqlite, Post>(sql_expr)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?;
        debug!("geted {} post from local", posts.len());
        Ok(posts)
    }

    async fn mark_post_unfavorited(id: i64, db: &SqlitePool) -> Result<()> {
        debug!("unfav post {} in db", id);
        sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn mark_post_favorited(id: i64, db: &SqlitePool) -> Result<()> {
        debug!("mark favorited post {} in db", id);
        sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn query_posts_to_unfavorite(db: &SqlitePool) -> Result<Vec<i64>> {
        debug!("query all posts to unfavorite");
        Ok(sqlx::query_as::<Sqlite, (i64,)>(
            "SELECT id FROM posts WHERE unfavorited == false and favorited;",
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|t| t.0)
        .collect())
    }

    pub async fn query_favorited_sum(db: &SqlitePool) -> Result<u32> {
        Ok(
            sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
                .fetch_one(db)
                .await?
                .0,
        )
    }

    pub async fn unfavorite_post(id: i64, db: &SqlitePool, fetcher: &WebFetcher) -> Result<()> {
        let idstr = id.to_string();
        let res = fetcher
            .post(
                DESTROY_FAVORITES,
                fetcher.web_client(),
                &serde_json::json!({ "id": idstr }),
            )
            .await?;
        let status_code = res.status().as_u16();

        if !res.status().is_success() {
            let res_json = res.json::<Value>().await;
            if status_code == 400
                && res_json.is_ok()
                && res_json.unwrap()["message"] == "not your collection!"
            {
                warn!("post {} have been unfavorited", idstr);
            } else {
                warn!(
                    "cannot unfavorite post {}, with http code {}",
                    idstr, status_code
                );
            }
        }
        Self::mark_post_unfavorited(id, db).await?;
        Ok(())
    }

    pub async fn fetch_posts(uid: i64, page: u32, fetcher: &WebFetcher) -> Result<Vec<Post>> {
        let url = format!("{STATUSES_MY_MICRO_BLOG_API}?uid={uid}&page={page}");
        debug!("fetch meta page, url: {url}");
        let mut json: Value = fetcher.get(url, fetcher.web_client()).await?.json().await?;
        trace!("get json: {json:?}");
        if json["ok"] != 1 {
            Err(Error::ResourceGetFailed(format!(
                "fetched data is not ok: {json:?}"
            )))
        } else if let Value::Array(posts) = json["data"]["list"].take() {
            let posts = posts
                .into_iter()
                .map(|post| post.try_into())
                .collect::<Result<Vec<Post>>>()?;
            let posts = join_all(posts.into_iter().map(|post| async {
                Ok(post
                    .with_process_client_only(fetcher)
                    .await?
                    .with_process_long_text(fetcher)
                    .await?)
            }))
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
            Ok(posts)
        } else {
            Err(Error::MalFormat(
                "Posts should be a array, maybe api has changed".into(),
            ))
        }
    }

    pub async fn fetch_fav_posts(uid: i64, page: u32, fetcher: &WebFetcher) -> Result<Vec<Post>> {
        let url = format!("{FAVORITES_ALL_FAV_API}?uid={uid}&page={page}");
        debug!("fetch fav meta page, url: {url}");
        let mut posts: Value = fetcher.get(url, fetcher.web_client()).await?.json().await?;
        trace!("get json: {posts:?}");
        if posts["ok"] != 1 {
            Err(Error::ResourceGetFailed(format!(
                "fetched data is not ok: {posts:?}"
            )))
        } else if let Value::Array(posts) = posts["data"].take() {
            let posts = posts
                .into_iter()
                .map(|post| post.try_into())
                .collect::<Result<Vec<Post>>>()?;
            let posts = join_all(posts.into_iter().map(|post| async {
                Ok(post
                    .with_process_client_only(fetcher)
                    .await?
                    .with_process_long_text(fetcher)
                    .await?)
            }))
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
            Ok(posts)
        } else {
            Err(Error::MalFormat(
                "Posts should be a array, maybe api has changed".into(),
            ))
        }
    }

    pub async fn with_process_long_text(mut self, fetcher: &WebFetcher) -> Result<Post> {
        if self.is_long_text {
            let content = LongText::fetch_long_text(&self.mblogid, fetcher).await?;
            self.text_raw = content;
        }
        Ok(self)
    }

    pub async fn with_process_client_only(mut self, fetcher: &WebFetcher) -> Result<Post> {
        if self.client_only {
            self = Self::fetch_mobile_page(&self.mblogid, fetcher).await?;
        }
        Ok(self)
    }

    pub async fn fetch_mobile_page(mblogid: &str, fetcher: &WebFetcher) -> Result<Post> {
        // let mobile_client = &self.mobile_client;
        let url = format!("{}{}", MOBILE_POST_API, mblogid);
        info!("fetch client only post url: {}", &url);
        let mut res: Value = fetcher
            .get(url, fetcher.mobile_client())
            .await?
            .json()
            .await?;
        if res["ok"] == 1 {
            // let post = Self::convert_mobile2pc_post(res["data"].take())?;
            let post = res["data"].take().try_into()?;
            Ok(post)
        } else {
            Err(Error::ResourceGetFailed(format!(
                "fetch mobile post {} failed, with message {}",
                mblogid, res["message"]
            )))
        }
    }

    pub async fn persist_posts(
        posts: Vec<Post>,
        with_pic: bool,
        image_definition: u8,
        db: &SqlitePool,
        fetcher: &WebFetcher,
    ) -> Result<()> {
        if with_pic {
            let emojis = posts
                .iter()
                .flat_map(|post| post.extract_emoji_urls().into_iter())
                .map(Picture::emoji);
            let avatar = posts.iter().filter_map(|post| {
                post.user
                    .as_ref()
                    .map(|user| user.get_avatar_pic(image_definition))
            });
            join_all(
                posts
                    .iter()
                    .flat_map(|post| {
                        post.extract_pic_urls(image_definition)
                            .into_iter()
                            .map(|url| Picture::in_post(url, post.id))
                    })
                    .chain(emojis)
                    .chain(avatar)
                    .map(|pic| async move { pic.persist(db, fetcher).await }),
            )
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        }

        join_all(
            posts
                .into_iter()
                .map(|post| async move { post.insert(db).await }),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

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
        let id = value_as_str(post, "id")?;
        let id = match id.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                return Err(Error::MalFormat(format!(
                    "failed to parse mobile post id {id}: {e}"
                )))
            }
        };
        post["id"] = Value::Number(serde_json::Number::from(id));
        post["mblogid"] = post["bid"].take();
        post["text_raw"] = post["text"].to_owned();
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
                    .map(|pic| value_as_str(pic, "pid"))
                    .collect::<Result<Vec<_>>>()?,
            )
            .unwrap();
            post["pic_infos"] = serde_json::to_value(
                pics.into_iter()
                    .map(|mut pic| {
                        let id = value_as_str(&pic, "pid")?.to_owned();
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
            Post::pic_ids_to_urls(pic_ids, pic_infos, image_definition)
        } else {
            Default::default()
        };
        if let Some(retweeted_post) = &self.retweeted_post {
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
                    .filter_map(|id| Post::select_pic_url(&pic_infos[id], image_definition))
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
                            + Post::trans_user(&text[m.start()..m.end()]),
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
                        acc + &text[i..m.start()] + Post::trans_topic(&text[m.start()..m.end()]),
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

    pub async fn generate_html(
        posts: Vec<Post>,
        html_name: &str,
        image_definition: u8,
        db: &SqlitePool,
        fetcher: &WebFetcher,
    ) -> Result<HTMLPage> {
        debug!("generate html from {} posts", posts.len());
        let mut pic_to_fetch = HashSet::new();
        let posts = posts
            .into_iter()
            .map(|post| {
                post.to_tera_context_val(
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
            if let Some(blob) = pic.get_blob(db, fetcher).await? {
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
        Err(e) => Err(Error::MalFormat(format!("{e}"))),
    }
}

fn extract_urls(text: &str) -> Vec<&str> {
    URL_EXPR.find_iter(text).map(|m| m.as_str()).collect()
}

// to be removed
fn value_as_str<'a>(v: &'a Value, property: &'a str) -> Result<&'a str> {
    v[property].as_str().ok_or(Error::MalFormat(format!(
        "property {} of {} cannot convert to str",
        property, v
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_datetime() {
        parse_created_at("Mon May 29 19:29:32 +0800 2023").unwrap();
        parse_created_at("Mon May 29 19:45:00 +0800 2023").unwrap();
        parse_created_at("Tue May 30 04:07:49 +0800 2023").unwrap();
    }
}
