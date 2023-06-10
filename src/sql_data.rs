use serde::Serialize;
use serde_json::from_value;
use sqlx::FromRow;

use crate::data::{Post, User};

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct SqlPost {
    pub id: i64,
    pub created_at: String,
    pub mblogid: String,
    pub text_raw: String,
    pub source: String,
    pub region_name: Option<String>,
    pub deleted: bool,
    pub uid: Option<i64>,
    pub pic_ids: Option<String>,
    pub pic_num: Option<i64>,
    pub retweeted_status: Option<i64>,
    pub url_struct: Option<String>,
    pub topic_struct: Option<String>,
    pub tag_struct: Option<String>,
    pub number_display_strategy: Option<String>,
    pub mix_media_info: Option<String>,
    #[sqlx(default)]
    pub visible: String,
    #[sqlx(default)]
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
    #[sqlx(default)]
    pub can_edit: bool,
    #[sqlx(default)]
    pub is_paid: bool,
    #[sqlx(default)]
    pub share_repost_type: Option<i64>,
    #[sqlx(default)]
    pub rid: Option<String>,
    #[sqlx(default)]
    pub pic_infos: Option<String>,
    #[sqlx(default)]
    pub cardid: Option<String>,
    #[sqlx(default)]
    pub pic_bg_new: Option<String>,
    #[sqlx(default)]
    pub mark: Option<String>,
    #[sqlx(default)]
    pub mblog_vip_type: Option<i64>,
    #[sqlx(default)]
    pub reposts_count: Option<i64>,
    #[sqlx(default)]
    pub comments_count: Option<i64>,
    #[sqlx(default)]
    pub attitudes_count: Option<i64>,
    #[sqlx(default)]
    pub mlevel: Option<i64>,
    #[sqlx(default)]
    pub content_auth: Option<i64>,
    #[sqlx(default)]
    pub is_show_bulletin: Option<i64>,
    #[sqlx(default)]
    pub repost_type: Option<i64>,
    #[sqlx(default)]
    pub edit_count: Option<i64>,
    #[sqlx(default)]
    pub mblogtype: Option<i64>,
    #[sqlx(default)]
    pub text_length: Option<i64>,
    #[sqlx(default, rename = "isLongText")]
    pub is_long_text: bool,
    #[sqlx(default)]
    pub annotations: Option<String>,
    #[sqlx(default)]
    pub geo: Option<String>,
    #[sqlx(default)]
    pub pic_focus_point: Option<String>,
    #[sqlx(default)]
    pub page_info: Option<String>,
    #[sqlx(default)]
    pub title: Option<String>,
    #[sqlx(default)]
    pub continue_tag: Option<String>,
    #[sqlx(default)]
    pub comment_manage_info: Option<String>,
}

impl From<Post> for SqlPost {
    fn from(p: Post) -> Self {
        let Post(mut p) = p;
        SqlPost {
            id: from_value(p["id"].take()).ok().unwrap_or_default(),
            created_at: from_value(p["created_at"].take()).ok().unwrap_or_default(),
            mblogid: from_value(p["mblogid"].take()).ok().unwrap_or_default(),
            text_raw: from_value(p["text_raw"].take()).ok().unwrap_or_default(),
            source: from_value(p["source"].take()).ok().unwrap_or_default(),
            region_name: from_value(p["region_name"].take()).ok().unwrap_or_default(),
            deleted: from_value(p["deleted"].take()).ok().unwrap_or_default(),
            uid: from_value(p["uid"].take()).ok().unwrap_or_default(),
            pic_ids: from_value(p["pic_ids"].take()).ok().unwrap_or_default(),
            pic_num: from_value(p["pic_num"].take()).ok().unwrap_or_default(),
            retweeted_status: from_value(p["retweeted_status"].take())
                .ok()
                .unwrap_or_default(),
            url_struct: from_value(p["url_struct"].take()).ok().unwrap_or_default(),
            topic_struct: from_value(p["topic_struct"].take())
                .ok()
                .unwrap_or_default(),
            tag_struct: from_value(p["tag_struct"].take()).ok().unwrap_or_default(),
            number_display_strategy: from_value(p["number_display_strategy"].take())
                .ok()
                .unwrap_or_default(),
            mix_media_info: from_value(p["mix_media_info"].take())
                .ok()
                .unwrap_or_default(),
            visible: from_value(p["visible"].take()).ok().unwrap_or_default(),
            text: from_value(p["text"].take()).ok().unwrap_or_default(),
            attitudes_status: from_value(p["attitudes_status"].take())
                .ok()
                .unwrap_or_default(),
            show_feed_repost: from_value(p["show_feed_repost"].take())
                .ok()
                .unwrap_or_default(),
            show_feed_comment: from_value(p["show_feed_comment"].take())
                .ok()
                .unwrap_or_default(),
            picture_viewer_sign: from_value(p["picture_viewer_sign"].take())
                .ok()
                .unwrap_or_default(),
            show_picture_viewer: from_value(p["show_picture_viewer"].take())
                .ok()
                .unwrap_or_default(),
            favorited: from_value(p["favorited"].take()).ok().unwrap_or_default(),
            can_edit: from_value(p["can_edit"].take()).ok().unwrap_or_default(),
            is_paid: from_value(p["is_paid"].take()).ok().unwrap_or_default(),
            share_repost_type: from_value(p["share_repost_type"].take())
                .ok()
                .unwrap_or_default(),
            rid: from_value(p["rid"].take()).ok().unwrap_or_default(),
            pic_infos: from_value(p["pic_infos"].take()).ok().unwrap_or_default(),
            cardid: from_value(p["cardid"].take()).ok().unwrap_or_default(),
            pic_bg_new: from_value(p["pic_bg_new"].take()).ok().unwrap_or_default(),
            mark: from_value(p["mark"].take()).ok().unwrap_or_default(),
            mblog_vip_type: from_value(p["mblog_vip_type"].take())
                .ok()
                .unwrap_or_default(),
            reposts_count: from_value(p["reposts_count"].take())
                .ok()
                .unwrap_or_default(),
            comments_count: from_value(p["comments_count"].take())
                .ok()
                .unwrap_or_default(),
            attitudes_count: from_value(p["attitudes_count"].take())
                .ok()
                .unwrap_or_default(),
            mlevel: from_value(p["mlevel"].take()).ok().unwrap_or_default(),
            content_auth: from_value(p["content_auth"].take())
                .ok()
                .unwrap_or_default(),
            is_show_bulletin: from_value(p["is_show_bulletin"].take())
                .ok()
                .unwrap_or_default(),
            repost_type: from_value(p["repost_type"].take()).ok().unwrap_or_default(),
            edit_count: from_value(p["edit_count"].take()).ok().unwrap_or_default(),
            mblogtype: from_value(p["mblogtype"].take()).ok().unwrap_or_default(),
            text_length: from_value(p["text_length"].take()).ok().unwrap_or_default(),
            is_long_text: from_value(p["is_long_text"].take())
                .ok()
                .unwrap_or_default(),
            annotations: from_value(p["annotations"].take()).ok().unwrap_or_default(),
            geo: from_value(p["geo"].take()).ok().unwrap_or_default(),
            pic_focus_point: from_value(p["pic_focus_point"].take())
                .ok()
                .unwrap_or_default(),
            page_info: from_value(p["page_info"].take()).ok().unwrap_or_default(),
            title: from_value(p["title"].take()).ok().unwrap_or_default(),
            continue_tag: from_value(p["continue_tag"].take())
                .ok()
                .unwrap_or_default(),
            comment_manage_info: from_value(p["comment_manage_info"].take())
                .ok()
                .unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct SqlUser {
    pub id: i64,
    pub profile_url: String,
    pub screen_name: String,
    pub profile_image_url: String,
    pub avatar_large: String,
    pub avatar_hd: String,
    #[sqlx(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    pub v_plus: i64,
    #[sqlx(default)]
    pub pc_new: i64,
    #[sqlx(default)]
    pub verified: bool,
    #[sqlx(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    pub domain: String,
    #[sqlx(default)]
    pub weihao: String,
    #[sqlx(default)]
    pub verified_type_ext: Option<i64>,
    #[sqlx(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    pub following: bool,
    #[sqlx(default)]
    pub mbrank: i64,
    #[sqlx(default)]
    pub mbtype: i64,
    #[sqlx(default)]
    pub icon_list: String,
}

impl From<User> for SqlUser {
    fn from(u: User) -> Self {
        let User(mut u) = u;
        SqlUser {
            id: from_value(u["id"].take()).ok().unwrap_or_default(),
            profile_url: from_value(u["profile_url"].take()).ok().unwrap_or_default(),
            screen_name: from_value(u["screen_name"].take()).ok().unwrap_or_default(),
            profile_image_url: from_value(u["profile_image_url"].take())
                .ok()
                .unwrap_or_default(),
            avatar_large: from_value(u["avatar_large"].take())
                .ok()
                .unwrap_or_default(),
            avatar_hd: from_value(u["avatar_hd"].take()).ok().unwrap_or_default(),
            planet_video: from_value(u["planet_video"].take())
                .ok()
                .unwrap_or_default(),
            v_plus: from_value(u["v_plus"].take()).ok().unwrap_or_default(),
            pc_new: from_value(u["pc_new"].take()).ok().unwrap_or_default(),
            verified: from_value(u["verified"].take()).ok().unwrap_or_default(),
            verified_type: from_value(u["verified_type"].take())
                .ok()
                .unwrap_or_default(),
            domain: from_value(u["domain"].take()).ok().unwrap_or_default(),
            weihao: from_value(u["weihao"].take()).ok().unwrap_or_default(),
            verified_type_ext: from_value(u["verified_type_ext"].take())
                .ok()
                .unwrap_or_default(),
            follow_me: from_value(u["follow_me"].take()).ok().unwrap_or_default(),
            following: from_value(u["following"].take()).ok().unwrap_or_default(),
            mbrank: from_value(u["mbrank"].take()).ok().unwrap_or_default(),
            mbtype: from_value(u["mbtype"].take()).ok().unwrap_or_default(),
            icon_list: from_value(u["icon_list"].take()).ok().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct PictureBlob {
    pub url: String,
    pub id: String,
    pub blob: Vec<u8>,
}

#[cfg(test)]
mod sql_data_test {
    use crate::data::{Post, User};
    use serde_json::{from_str, Value};

    use super::{SqlPost, SqlUser};

    #[test]
    fn from_post() {
        let post = Post(from_str(include_str!("../res/one.json")).unwrap());
        let _ = SqlPost::from(post);
    }

    #[test]
    fn from_posts() {
        let mut posts: Value = from_str(include_str!("../res/full.json")).unwrap();
        let posts = if let Value::Array(posts) = posts["data"].take() {
            posts
        } else {
            vec![]
        };
        posts.into_iter().for_each(|p| {
            let _ = SqlPost::from(Post(p));
        });
    }

    #[test]
    fn from_users() {
        let mut posts: Value = from_str(include_str!("../res/full.json")).unwrap();
        let posts = if let Value::Array(posts) = posts["data"].take() {
            posts
        } else {
            vec![]
        };
        posts.into_iter().for_each(|mut p| {
            if !p["user"]["id"].is_null() {
                let user = User(p["user"].take());
                let _ = SqlUser::from(user);
            }
        })
    }
}
