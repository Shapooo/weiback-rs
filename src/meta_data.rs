use log::{info, trace};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Posts {
    pub data: Vec<Post>,
    pub ok: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Post {
    pub id: u64,
    pub visible: Value,
    pub created_at: String,
    pub mblogid: String,
    #[serde(deserialize_with = "deserialize_post_user")]
    pub user: Option<PostUser>,
    pub text_raw: String,
    pub text: String,
    pub attitudes_status: i64,
    pub share_repost_type: Option<i64>,
    #[serde(rename = "showFeedRepost")]
    pub show_feed_repost: bool,
    #[serde(rename = "showFeedComment")]
    pub show_feed_comment: bool,
    #[serde(rename = "pictureViewerSign")]
    pub picture_viewer_sign: bool,
    #[serde(rename = "showPictureViewer")]
    pub show_picture_viewer: bool,
    // #[serde(rename = "customIcons")]
    // custom_icons: Vec<String>,
    // #[serde(rename = "rcList")]
    // rc_list: Vec<String>,
    // tags: String,
    pub source: String,
    #[serde(default)]
    pub favorited: bool,
    #[serde(default)]
    pub can_edit: bool,
    #[serde(default)]
    pub annotations: Value,
    pub rid: Option<String>,
    pub cardid: Option<String>,
    pub pic_ids: Option<Vec<String>>,
    #[serde(default)]
    pub pic_infos: Value,
    #[serde(default)]
    pub geo: Value,
    pub pic_num: Option<i64>,
    #[serde(default)]
    pub pic_focus_point: Value,
    #[serde(default)]
    pub is_paid: bool,
    pub pic_bg_new: Option<String>,
    #[serde(default)]
    pub topic_struct: Value,
    #[serde(default)]
    pub page_info: Value,
    #[serde(deserialize_with = "deserialize_str_bool")]
    #[serde(default)]
    pub deleted: bool,
    pub mark: Option<String>,
    #[serde(default)]
    pub tag_struct: Value,
    #[serde(default)]
    pub title: Value,
    pub mblog_vip_type: Option<i64>,
    #[serde(default)]
    pub number_display_strategy: Value,
    pub reposts_count: Option<i64>,
    pub comments_count: Option<i64>,
    pub attitudes_count: Option<i64>,
    #[serde(default)]
    pub continue_tag: Value,
    pub mlevel: Option<i64>,
    pub content_auth: Option<i64>,
    pub is_show_bulletin: Option<i64>,
    #[serde(default)]
    pub comment_manage_info: Value,
    pub repost_type: Option<i64>,
    #[serde(default)]
    pub url_struct: Value,
    pub retweeted_status: Option<Box<Post>>,
    pub edit_count: Option<i64>,
    pub mblogtype: Option<i64>,
    pub region_name: Option<String>,
    #[serde(default)]
    pub mix_media_info: Value,
    #[serde(rename = "textLength")]
    pub text_length: Option<i64>,
    #[serde(default, rename = "isLongText")]
    pub is_long_text: bool,
}

impl Post {
    pub fn to_sql(&self) -> String {
        let mut sql = String::from("INSERT OR IGNORE INTO fav_post (");
        let mut values = String::from(") VALUES (");

        sql.push_str("id");
        values.push_str(self.id.to_string().as_str());
        sql.push_str(",visible");
        values.push(',');
        values.push_str(&to_sql_str(self.visible.to_string().as_str()));
        sql.push_str(",created_at");
        values.push(',');
        values.push_str(to_sql_str(&self.created_at).as_str());
        sql.push_str(",mblogid");
        values.push(',');
        values.push_str(to_sql_str(&self.mblogid).as_str());
        if let Some(user) = &self.user {
            sql.push_str(",uid");
            values.push(',');
            values.push_str(user.id.to_string().as_str());
        }
        sql.push_str(",text_raw");

        values.push(',');
        values.push_str(to_sql_str(&self.text_raw).as_str());
        sql.push_str(",text");
        values.push(',');
        values.push_str(to_sql_str(&self.text).as_str());
        sql.push_str(",attitudes_status");
        values.push(',');
        values.push_str(self.attitudes_status.to_string().as_str());
        if let Some(share_repost_type) = self.share_repost_type {
            sql.push_str(",share_repost_type");
            values.push(',');
            values.push_str(share_repost_type.to_string().as_str());
        }
        sql.push_str(",showFeedRepost");
        values.push(',');
        values.push_str(self.show_feed_repost.to_string().as_str());
        sql.push_str(",showFeedComment");
        values.push(',');
        values.push_str(self.show_feed_comment.to_string().as_str());
        sql.push_str(",pictureViewerSign");
        values.push(',');
        values.push_str(self.picture_viewer_sign.to_string().as_str());
        sql.push_str(",showPictureViewer");
        values.push(',');
        values.push_str(self.show_picture_viewer.to_string().as_str());
        sql.push_str(",source");
        values.push(',');
        values.push_str(to_sql_str(&self.source).as_str());
        sql.push_str(",favorited");
        values.push(',');
        values.push_str(self.favorited.to_string().as_str());
        sql.push_str(",can_edit");
        values.push(',');
        values.push_str(self.can_edit.to_string().as_str());
        sql.push_str(",annotations");
        values.push(',');
        values.push_str(&to_sql_str(self.annotations.to_string().as_str()));
        if let Some(rid) = &self.rid {
            sql.push_str(",rid");
            values.push(',');
            values.push_str(to_sql_str(rid).as_str());
        }
        if let Some(cardid) = &self.cardid {
            sql.push_str(",cardid");
            values.push(',');
            values.push_str(to_sql_str(cardid).as_str());
        }
        if let Some(pic_ids) = &self.pic_ids {
            sql.push_str(",pic_ids");
            values.push(',');
            let mut tmp_str = String::from("'");
            pic_ids.iter().for_each(|id| {
                tmp_str.push_str(id);
                tmp_str.push('|')
            });
            tmp_str.push('\'');
            values.push_str(tmp_str.as_str());
        }
        sql.push_str(",pic_infos");
        values.push(',');
        values.push_str(&to_sql_str(self.pic_infos.to_string().as_str()));
        sql.push_str(",geo");
        values.push(',');
        values.push_str(&to_sql_str(self.geo.to_string().as_str()));
        if let Some(pic_num) = self.pic_num {
            sql.push_str(",pic_num");
            values.push(',');
            values.push_str(pic_num.to_string().as_str());
        }
        sql.push_str(",pic_focus_point");
        values.push(',');
        values.push_str(&to_sql_str(self.pic_focus_point.to_string().as_str()));
        sql.push_str(",is_paid");
        values.push(',');
        values.push_str(self.is_paid.to_string().as_str());
        if let Some(pic_bg_new) = &self.pic_bg_new {
            sql.push_str(",pic_bg_new");
            values.push(',');
            values.push_str(to_sql_str(pic_bg_new).as_str());
        }
        sql.push_str(",topic_struct");
        values.push(',');
        values.push_str(&to_sql_str(self.topic_struct.to_string().as_str()));
        sql.push_str(",page_info");
        values.push(',');
        values.push_str(&to_sql_str(self.page_info.to_string().as_str()));
        sql.push_str(",deleted");
        values.push(',');
        values.push_str(self.deleted.to_string().as_str());
        if let Some(mark) = &self.mark {
            sql.push_str(",mark");
            values.push(',');
            values.push_str(to_sql_str(mark).as_str());
        }
        sql.push_str(",tag_struct");
        values.push(',');
        values.push_str(&to_sql_str(self.tag_struct.to_string().as_str()));
        sql.push_str(",title");
        values.push(',');
        values.push_str(&to_sql_str(self.title.to_string().as_str()));
        if let Some(mblog_vip_type) = self.mblog_vip_type {
            sql.push_str(",mblog_vip_type");
            values.push(',');
            values.push_str(mblog_vip_type.to_string().as_str());
        }
        sql.push_str(",number_display_strategy");
        values.push(',');
        values.push_str(&to_sql_str(self.number_display_strategy.to_string().as_str()));
        if let Some(reposts_count) = self.reposts_count {
            sql.push_str(",reposts_count");
            values.push(',');
            values.push_str(reposts_count.to_string().as_str());
        }
        if let Some(comments_count) = self.comments_count {
            sql.push_str(",comments_count");
            values.push(',');
            values.push_str(comments_count.to_string().as_str());
        }
        if let Some(attitudes_count) = self.attitudes_count {
            sql.push_str(",attitudes_count");
            values.push(',');
            values.push_str(attitudes_count.to_string().as_str());
        }
        sql.push_str(",continue_tag");
        values.push(',');
        values.push_str(&to_sql_str(self.continue_tag.to_string().as_str()));
        if let Some(mlevel) = self.mlevel {
            sql.push_str(",mlevel");
            values.push(',');
            values.push_str(mlevel.to_string().as_str());
        }
        if let Some(content_auth) = self.content_auth {
            sql.push_str(",content_auth");
            values.push(',');
            values.push_str(content_auth.to_string().as_str());
        }
        if let Some(is_show_bulletin) = self.is_show_bulletin {
            sql.push_str(",is_show_bulletin");
            values.push(',');
            values.push_str(is_show_bulletin.to_string().as_str());
        }
        sql.push_str(",comment_manage_info");
        values.push(',');
        values.push_str(&to_sql_str(self.comment_manage_info.to_string().as_str()));
        if let Some(repost_type) = self.repost_type {
            sql.push_str(",repost_type");
            values.push(',');
            values.push_str(repost_type.to_string().as_str());
        }
        sql.push_str(",url_struct");
        values.push(',');
        values.push_str(&to_sql_str(self.url_struct.to_string().as_str()));
        if let Some(retweeted_status) = &self.retweeted_status {
            sql.push_str(",retweeted_status");
            values.push(',');
            values.push_str(retweeted_status.id.to_string().as_str());
        }
        if let Some(edit_count) = self.edit_count {
            sql.push_str(",edit_count");
            values.push(',');
            values.push_str(edit_count.to_string().as_str());
        }
        if let Some(mblogtype) = self.mblogtype {
            sql.push_str(",mblogtype");
            values.push(',');
            values.push_str(mblogtype.to_string().as_str());
        }
        if let Some(region_name) = &self.region_name {
            sql.push_str(",region_name");
            values.push(',');
            values.push_str(to_sql_str(region_name).as_str());
        }
        sql.push_str(",mix_media_info");
        values.push(',');
        values.push_str(&to_sql_str(self.mix_media_info.to_string().as_str()));
        if let Some(text_length) = self.text_length {
            sql.push_str(",textLength");
            values.push(',');
            values.push_str(text_length.to_string().as_str());
        }
        sql.push_str(",isLongText");
        values.push(',');
        values.push_str(self.is_long_text.to_string().as_str());

        sql.push_str(&values);
        sql.push(')');
        trace!("{sql}");
        sql
    }
}

fn to_sql_str(s: &str) -> String {
    let mut s = s.replace('\'', "''");
    s.insert(0, '\'');
    s.push('\'');
    s
}

impl PostUser {
    pub fn to_sql(&self) -> String {
        let mut sql = String::from("INSERT OR IGNORE INTO user (");
        let mut values = String::from(") VALUES (");

        sql.push_str("profile_url");
        values.push_str(format!("{:?}", self.profile_url).as_str());
        sql.push_str(",planet_video");
        values.push(',');
        values.push_str(self.planet_video.to_string().as_str());
        sql.push_str(",v_plus");
        values.push(',');
        values.push_str(self.v_plus.to_string().as_str());
        sql.push_str(",id");
        values.push(',');
        values.push_str(self.id.to_string().as_str());
        sql.push_str(",pc_new");
        values.push(',');
        values.push_str(self.pc_new.to_string().as_str());
        sql.push_str(",screen_name");
        values.push(',');
        values.push_str(format!("{:?}", self.screen_name).as_str());
        sql.push_str(",profile_image_url");
        values.push(',');
        values.push_str(format!("{:?}", self.profile_image_url).as_str());
        sql.push_str(",verified");
        values.push(',');
        values.push_str(self.verified.to_string().as_str());
        sql.push_str(",verified_type");
        values.push(',');
        values.push_str(self.verified_type.to_string().as_str());
        sql.push_str(",domain");
        values.push(',');
        values.push_str(format!("{:?}", self.domain).as_str());
        sql.push_str(",weihao");
        values.push(',');
        values.push_str(format!("{:?}", self.weihao).as_str());
        if let Some(verified_type_ext) = &self.verified_type_ext {
            sql.push_str(",verified_type_ext");
            values.push(',');
            values.push_str(verified_type_ext.to_string().as_str());
        }
        sql.push_str(",avatar_large");
        values.push(',');
        values.push_str(format!("{:?}", self.avatar_large).as_str());
        sql.push_str(",avatar_hd");
        values.push(',');
        values.push_str(format!("{:?}", self.avatar_hd).as_str());
        sql.push_str(",follow_me");
        values.push(',');
        values.push_str(self.follow_me.to_string().as_str());
        sql.push_str(",following");
        values.push(',');
        values.push_str(self.following.to_string().as_str());
        sql.push_str(",mbrank");
        values.push(',');
        values.push_str(self.mbrank.to_string().as_str());
        sql.push_str(",mbtype");
        values.push(',');
        values.push_str(self.mbtype.to_string().as_str());
        sql.push_str(",icon_list");
        values.push(',');
        values.push('\'');
        values.push_str(self.icon_list.to_string().as_str());
        values.push('\'');

        values.push(')');
        sql.push_str(&values);
        trace!("{}", sql);
        sql
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostUser {
    pub profile_url: String,
    pub planet_video: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub v_plus: i64,
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub pc_new: i64,
    #[serde(default)]
    pub screen_name: String,
    #[serde(default)]
    pub profile_image_url: String,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub verified_type: i64,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub weihao: String,
    #[serde(default)]
    pub verified_type_ext: Option<i64>,
    #[serde(default)]
    pub avatar_large: String,
    #[serde(default)]
    pub avatar_hd: String,
    #[serde(default)]
    pub follow_me: bool,
    #[serde(default)]
    pub following: bool,
    #[serde(default)]
    pub mbrank: i64,
    #[serde(default)]
    pub mbtype: i64,
    #[serde(default)]
    pub icon_list: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FavTag {
    pub fav_total_num: u64,
    pub ok: u8,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn deserialize_post_user<'de, D>(deserializer: D) -> Result<Option<PostUser>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut user: Option<PostUser> = Option::deserialize(deserializer)?;
    if let Some(user) = user.take() {
        if user.id == 0 {
            return Ok(None);
        } else {
            return Ok(Some(user));
        }
    } else {
        return Ok(None);
    }
}

fn deserialize_str_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if s.is_none() {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::meta_data::{Post, Posts};

    #[test]
    fn parse_post() {
        let txt = include_str!("../.tmp/one.json");
        serde_json::from_str::<Post>(txt).unwrap();
        // dbg!(a);
        // assert!(a.is_ok());
    }

    #[test]
    fn parse_posts() {
        let txt = include_str!("../.tmp/full.json");
        serde_json::from_str::<Posts>(txt).unwrap();
        // assert!(a.is_ok());
    }

    #[test]
    fn post_sql() {
        let txt = include_str!("../.tmp/one.json");
        let post = serde_json::from_str::<Post>(txt).unwrap();
        print!("{}", post.to_sql());
    }

    #[test]
    fn posts_sql() {
        let txt = include_str!("../.tmp/full.json");
        let posts = serde_json::from_str::<Posts>(txt).unwrap();
        posts.data.iter().for_each(|p| {
            p.to_sql();
        });
    }

    #[test]
    fn user_sql() {
        let txt = include_str!("../.tmp/one.json");
        let post = serde_json::from_str::<Post>(txt).unwrap();
        print!("{}", post.user.unwrap().to_sql());
        // assert!(a.is_ok());
    }
}
