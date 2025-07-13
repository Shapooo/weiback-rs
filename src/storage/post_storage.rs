use std::ops::DerefMut;

use anyhow::Result;
use serde_json::Value;
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use weibosdk_rs::User;

use crate::models::Picture;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PostStorage {
    pub id: i64,
    pub mblogid: String,
    pub text_raw: String,
    pub source: String,
    pub region_name: Option<String>,
    pub deleted: bool,
    pub pic_ids: Option<Value>,
    pub pic_num: Option<i64>,
    pub url_struct: Option<Value>,
    pub topic_struct: Option<Value>,
    pub tag_struct: Option<Value>,
    pub tags: Option<Value>,
    #[sqlx(rename = "customIcons")]
    pub custom_icons: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub visible: Value,
    pub text: String,
    #[sqlx(default)]
    pub attitudes_status: i64,
    #[sqlx(default, rename = "showFeedRepost")]
    pub show_feed_repost: bool,
    #[sqlx(default, rename = "showFeedComment")]
    pub show_feed_comment: bool,
    #[sqlx(default, rename = "pictureViewerSign")]
    pub picture_viewer_sign: bool,
    #[sqlx(default, rename = "showPictureViewer")]
    pub show_picture_viewer: bool,
    #[sqlx(default)]
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
    pub text_length: Option<i64>,
    #[sqlx(default, rename = "isLongText")]
    pub is_long_text: bool,
    #[sqlx(rename = "rcList")]
    pub rc_list: Option<Value>,
    pub annotations: Option<Value>,
    pub geo: Option<Value>,
    pub pic_focus_point: Option<Value>,
    pub page_info: Option<Value>,
    pub title: Option<Value>,
    pub continue_tag: Option<Value>,
    pub comment_manage_info: Option<Value>,
    #[sqlx(default)]
    pub client_only: bool,
    #[sqlx(default)]
    pub unfavorited: bool,
    pub created_at: String,
    pub created_at_timestamp: i64,
    pub created_at_tz: String,
    #[sqlx(skip)]
    pub retweeted_status: Option<Box<PostStorage>>,
    #[sqlx(skip)]
    pub user: Option<User>,
}

impl PostStorage {
    pub async fn create_table<E>(mut executor: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
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
             retweeted_id INTEGER, \
             url_struct TEXT, \
             topic_struct TEXT, \
             tag_struct TEXT, \
             tags TEXT, \
             customIcons TEXT, \
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
             complaint TEXT, \
             content_auth INTEGER, \
             is_show_bulletin INTEGER, \
             repost_type INTEGER, \
             edit_count INTEGER, \
             mblogtype INTEGER, \
             textLength INTEGER, \
             isLongText INTEGER, \
             rcList TEXT, \
             annotations TEXT, \
             geo TEXT, \
             pic_focus_point TEXT, \
             page_info TEXT, \
             title TEXT, \
             continue_tag TEXT, \
             comment_manage_info TEXT, \
             client_only INTEGER, \
             unfavorited INTEGER, \
             created_at TEXT, \
             created_at_timestamp INTEGER, \
             created_at_tz TEXT \
             )",
        )
        .execute(&mut *executor)
        .await?;
        Ok(())
    }
    pub async fn persist_posts(
        posts: Vec<PostStorage>,
        with_pic: bool,
        image_definition: u8,
        db: &SqlitePool,
        fetcher: &WebFetcher,
    ) -> Result<()> {
        let mut trans = db.begin().await?;
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
            for pic in posts
                .iter()
                .flat_map(|post| {
                    post.extract_pic_urls(image_definition)
                        .into_iter()
                        .map(|url| Picture::in_post(url, post.id))
                })
                .chain(emojis)
                .chain(avatar)
            {
                pic.persist(trans.as_mut(), fetcher).await?;
            }
        }

        for post in posts {
            post.insert(trans.as_mut()).await?;
        }
        trans.commit().await?;

        Ok(())
    }
}
