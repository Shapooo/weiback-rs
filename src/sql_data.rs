use crate::fetched_data::{FetchedPost, FetchedUser};
use sqlx::FromRow;
#[derive(Debug, Clone, FromRow)]
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

impl From<FetchedPost> for SqlPost {
    fn from(p: FetchedPost) -> Self {
        SqlPost {
            id: p.id as i64,
            created_at: p.created_at,
            mblogid: p.mblogid,
            text_raw: p.text_raw,
            source: p.source,
            region_name: p.region_name,
            deleted: p.deleted,
            uid: p.user.map(|user| user.id as i64),
            pic_ids: p.pic_ids.map(|ids| ids.join("|")),
            pic_num: p.pic_num,
            retweeted_status: p.retweeted_status.map(|r| r.id as i64),
            url_struct: value_to_option_string(&p.url_struct),
            topic_struct: value_to_option_string(&p.topic_struct),
            tag_struct: value_to_option_string(&p.tag_struct),
            number_display_strategy: value_to_option_string(&p.number_display_strategy),
            mix_media_info: value_to_option_string(&p.mix_media_info),
            visible: p.visible.to_string(),
            text: p.text,
            attitudes_status: p.attitudes_status,
            show_feed_repost: p.show_feed_repost,
            show_feed_comment: p.show_feed_comment,
            picture_viewer_sign: p.picture_viewer_sign,
            show_picture_viewer: p.show_picture_viewer,
            favorited: p.favorited,
            can_edit: p.can_edit,
            is_paid: p.is_paid,
            share_repost_type: p.share_repost_type,
            rid: p.rid,
            pic_infos: value_to_option_string(&p.pic_infos),
            cardid: p.cardid,
            pic_bg_new: p.pic_bg_new,
            mark: p.mark,
            mblog_vip_type: p.mblog_vip_type,
            reposts_count: p.reposts_count,
            comments_count: p.comments_count,
            attitudes_count: p.attitudes_count,
            mlevel: p.mlevel,
            content_auth: p.content_auth,
            is_show_bulletin: p.is_show_bulletin,
            repost_type: p.share_repost_type,
            edit_count: p.edit_count,
            mblogtype: p.mblogtype,
            text_length: p.text_length,
            is_long_text: p.is_long_text,
            annotations: value_to_option_string(&p.annotations),
            geo: value_to_option_string(&p.geo),
            pic_focus_point: value_to_option_string(&p.pic_focus_point),
            page_info: value_to_option_string(&p.page_info),
            title: value_to_option_string(&p.title),
            continue_tag: value_to_option_string(&p.continue_tag),
            comment_manage_info: value_to_option_string(&p.comment_manage_info),
        }
    }
}

fn value_to_option_string(value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        None
    } else {
        Some(value.to_string())
    }
}

#[derive(Debug, Clone, FromRow)]
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

impl From<FetchedUser> for SqlUser {
    fn from(u: FetchedUser) -> Self {
        SqlUser {
            id: u.id as i64,
            profile_url: u.profile_url,
            screen_name: u.screen_name,
            profile_image_url: u.profile_image_url,
            avatar_large: u.avatar_large,
            avatar_hd: u.avatar_hd,
            planet_video: u.planet_video,
            v_plus: u.v_plus,
            pc_new: u.pc_new,
            verified: u.verified,
            verified_type: u.verified_type,
            domain: u.domain,
            weihao: u.weihao,
            verified_type_ext: u.verified_type_ext,
            follow_me: u.follow_me,
            following: u.following,
            mbrank: u.mbrank,
            mbtype: u.mbtype,
            icon_list: u.icon_list.to_string(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct PictureBlob {
    pub url: String,
    pub id: String,
    pub blob: Vec<u8>,
}
