use super::super::service::{emoticon::emoticon_get, search_args::SearchArgs};
use super::{picture::Picture, user::User};
use crate::exporter::{html_generator::HTMLGenerator, HTMLPage, HTMLPicture};

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
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};

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

/** 微博博文数据
 * 从微博 API 获取的每条 post 数据，原始数据为 Json 格式，包含如下字段：
 * annotations              json 列表，含义不明
 * attitudes_count          整型，猜测为点赞等的计数
 * attitudes_status         整形，常为0或1，应该为自己是否点赞，每个 post 都包含
 * buttons                  json 列表，应该为网页展示的按钮相关
 * can_edit                 布尔类型，自己是否可编辑，所有合法 post 都包含
 * cardid                   字符串，示例："vip007"
 * comment_manage_info      json 对象，示例：“{ "comment_permission_type": -1, "approval_comment_type": 0, "comment_sort_type": 0 }”，所有合法 post 都包含
 * comments_count           整型，所有合法 post 都包含
 * complaint                json 对象，猜测为微博辟谣，示例：“{ "url": "https://weibo.com/2638582553/Mzg6E7eeJ", "class": "1", "classdesc": "整体失实", "showcontent": "徐汇中学：女生进入男浴室不属实。网传音频经过编辑，与事实不符", "cmt_desc": "徐汇中学：女生进入男浴室不属实。网传音频经过编辑，与事实不符", "fwd_desc": "徐汇中学：女生进入男浴室不属实。网传音频经过编辑，与事实不符", "color": 2, "wx_content_url": "", "actionlog": { "act_code": 6528, "fid": null, "lfid": null, "uicode": "20000420", "luicode": null, "ext": "uid:1865990891|mid:4883891406771778|mlevel:0" } }”
 * content_auth             整型，猜测为微博内容是否经过认证，所有合法 post 都包含
 * continue_tag             json 对象，猜测为微博文章网页按钮相关，示例：“{ "title": "全文", "pic": "http://h5.sinaimg.cn/upload/2015/09/25/3/timeline_card_small_article.png", "scheme": "sinaweibo://detail?mblogid=4899791286306197&id=4899791286306197" }”
 * created_at               字符串，示例："Thu May 11 18:25:07 +0800 2023"，每个 post 都包含
 * customIcons              json 列表，都为空，后续可考虑删除，每个 post 都包含
 * deleted                  字符串，但值都为"1"，代码中将其改为 bool 类型
 * edit_count               整型，应该是修改次数
 * favorited                布尔类型，是否收藏
 * geo                      json 对象，地理位置，可为 null，示例：“{ "type": "Point", "coordinates": [ 31.174061, 121.372833 ] }”，所有合法 post 都包含
 * id                       整型，每个 post 都包含
 * idstr                    字符串，信息与上重复，代码中忽略，每个 post 都包含
 * isLongText               布尔类型，是否长文本，但这个可能不是长文本时返回是，需要在代码中绕过，所有合法 post 都包含
 * is_paid                  布尔类型，猜测推广相关，所有合法 post 都包含
 * is_show_bulletin         整型，所有合法 post 都包含
 * mark                     字符串，示例："999_reallog_mark_ad:999|WeiboADNatural"
 * mblog_vip_type           整型，多为0，所有合法 post 都包含
 * mblogid                  字符串，应该是另一种 id，每个 post 都包含
 * mblogtype                整型，所有合法 post 都包含
 * mid                      字符串，值与 idstr 相同，代码中忽略，每个 post 都包含
 * mix_media_info           json 对象，同时发送视频图片可能会触发这个
 * mlevel                   整型，所有合法 post 都包含
 * number_display_strategy  json 对象，示例：“{ "apply_scenario_flag": 3, "display_text_min_number": 1000000, "display_text": "100万+" }”
 * page_info                json 对象，可能和微博文章相关，示例：“{ "type": "23", "page_id": "2317162022_1413622_-_956ac0", "object_type": "hudongvote", "page_desc": "", "page_title": "Deep L是high dimension还是high order？", "page_pic": "https://h5.sinaimg.cn/upload/100/721/2019/03/14/vote.png", "type_icon": "", "page_url": "https://vote.weibo.com/h5/index/index?vote_id=2022_1413622_-_956ac0&from=1FFFF96039&weiboauthoruid=1655747731", "object_id": "1022:2317162022_1413622_-_956ac0", "actionlog": { "act_type": 1, "act_code": 300, "oid": "1022:2317162022_1413622_-_956ac0", "uuid": 4821281613807648, "cardid": "", "lcardid": "", "uicode": "20000420", "luicode": "", "fid": "", "lfid": "", "ext": "mid:4821281612498984|rid:13_0_0_5116892773669667031_0_0_0|short_url:http://t.cn/A6oUhdqu|long_url:https://vote.weibo.com/h5/index/index?vote_id=2022_1413622_-_956ac0|comment_id:|miduid:1668515321|rootmid:4821281612498984|rootuid:1655747731|authorid:|uuid:4821281613807648|is_ad_weibo:0|analysis_card:page_info" }”
 * pic_bg_new               字符串，url，应该为网页展示卡片小背景
 * pic_focus_point          json 列表，图片缩略展示的位置，示例：“[ { "focus_point": { "left": 0, "top": 0, "width": 0.834782600402832, "height": 0.6267682313919067 }, "pic_id": "6a59b68fly1h8kxv9bym3j20t112o7ac" }, { "focus_point": { "left": 0.04637681320309639, "top": 0, "width": 0.834782600402832, "height": 0.9427168369293213 }, "pic_id": "6a59b68fly1h8kxv3oqvyj20k00hqq4s" }, { "focus_point": { "left": 0.41449275612831116, "top": 0.39782607555389404, "width": 0.23333333432674408, "height": 0.10652174055576324 }, "pic_id": "6a59b68fly1h8kxum8813j20rs0ijgp4" } ]”
 * pic_ids                  json 列表，示例：“[ "63918611ly1hcs1yh885jj20zg0f3ahq" ]”
 * pic_infos                json 对象，示例：“{ "6d1b7657gy1hb7tcganh1j21400u0dl2": { "thumbnail": { "url": "https://wx4.sinaimg.cn/wap180/6d1b7657gy1hb7tcganh1j21400u0dl2.jpg", "width": 180, "height": 134, "cut_type": 1, "type": null }, "bmiddle": { "url": "https://wx4.sinaimg.cn/wap360/6d1b7657gy1hb7tcganh1j21400u0dl2.jpg", "width": 360, "height": 269, "cut_type": 1, "type": null }, "large": { "url": "https://wx4.sinaimg.cn/orj960/6d1b7657gy1hb7tcganh1j21400u0dl2.jpg", "width": 1280, "height": 960, "cut_type": 1, "type": null }, "original": { "url": "https://wx4.sinaimg.cn/orj1080/6d1b7657gy1hb7tcganh1j21400u0dl2.jpg", "width": 1440, "height": 1080, "cut_type": 1, "type": null }, "largest": { "url": "https://wx4.sinaimg.cn/large/6d1b7657gy1hb7tcganh1j21400u0dl2.jpg", "width": 1440, "height": 1080, "cut_type": 1, "type": null }, "mw2000": { "url": "https://wx4.sinaimg.cn/mw2000/6d1b7657gy1hb7tcganh1j21400u0dl2.jpg", "width": 1440, "height": 1080, "cut_type": 1, "type": null }, "focus_point": { "left": 0.17007707059383392, "top": 0.006124263163655996, "width": 0.6598458290100098, "height": 0.8789452314376831 }, "object_id": "1042018:13ba3b5df15134780a5776475d33faf0", "pic_id": "6d1b7657gy1hb7tcganh1j21400u0dl2", "photo_tag": 0, "type": "pic", "pic_status": 0 } },”
 * pic_num                  整型，所有合法 post 都包含
 * pictureViewerSign        布尔类型，每个 post 都包含
 * rcList                   json 列表，多为空，每个 post 都包含
 * region_name              字符串，示例："发布于 北京"
 * repost_type              整型
 * reposts_count            整型，所有合法 post 都包含
 * retweeted_status         json 对象，为转发的原微博
 * rid                      字符串，示例："9_0_0_3383423073694276665_0_0_0"，所有合法 post 都包含
 * share_repost_type        整型
 * showFeedComment          布尔类型，每个 post 都包含
 * showFeedRepost           布尔类型，每个 post 都包含
 * showPictureViewer        布尔类型，每个 post 都包含
 * source                   字符串，示例："微博网页版"，"<a target=\"_blank\" href=\"https://app.weibo.com/t/feed/Z5q8X\" rel=\"nofollow\">moto edge X30</a>"，每个 post 都包含
 * tag_struct               json 列表，示例：“[ { "tag_name": "魔法师蛋小丁的小店", "oid": "1042092:weibostore_2213561393", "tag_type": 2, "tag_hidden": 0, "tag_scheme": "sinaweibo://browser?showmenu=0&topnavstyle=1&immersiveScroll=50&url=https%3A%2F%2Fshop.sc.weibo.com%2Fh5%2Fredirect%2Fdispatcher%3Fextparam%3Dfrom%3Atag%26containerid%3D231439weibostore_2213561393%26_mid%3D4892763001193114%26_uicode%3D20000420%26extparam%3Dfrom%253Atag", "url_type_pic": "https://h5.sinaimg.cn/upload/1008/253/2020/11/04/wb_shop.png", "actionlog": { "act_code": 2413, "oid": "1042092:weibostore_2213561393", "uicode": "20000420", "luicode": null, "fid": null, "ext": "|tag_type:store" }, "bd_object_type": "store" } ]”
 * tags                     json 列表，多为空，可能是收藏标签
 * text                     字符串，带html标记的文本，每个 post 都包含
 * textLength               整型
 * text_raw                 字符串，不带格式的原文，每个 post 都包含
 * title                    字符串，多为"全文"
 * topic_struct             json 列表，示例：“[ { "title": "", "topic_url": "sinaweibo://searchall?containerid=231522&q=%23%E7%94%B5%E5%BD%B1%E7%81%8C%E7%AF%AE%E9%AB%98%E6%89%8B%23&extparam=%23%E7%94%B5%E5%BD%B1%E7%81%8C%E7%AF%AE%E9%AB%98%E6%89%8B%23", "topic_title": "电影灌篮高手", "actionlog": { "act_type": 1, "act_code": 300, "oid": "1022:23152204fbdb287850a6f4ff9819b1d21465b1", "uuid": 4786764060557448, "cardid": "", "lcardid": "", "uicode": "20000420", "luicode": "", "fid": "", "lfid": "", "ext": "mid:4892644125967414|rid:3_0_0_5226100761076638313_0_0_0|short_url:|long_url:|comment_id:|miduid:2492465520|rootmid:4892644125967414|rootuid:2492465520|authorid:|uuid:4786764060557448|is_ad_weibo:0" } } ]”
 * url_struct               json 列表，示例：“[ { "url_title": "字节一年，人间三年！！", "url_type_pic": "https://h5.sinaimg.cn/upload/2015/09/25/3/timeline_card_small_web.png", "ori_url": "sinaweibo://slidebrowser?url=https%3A%2F%2Fmp.weixin.qq.com%2Fs%2FjD8qwmUbfwWBd97K0F0NMg&oid=3000000041%3A2778e8aab17435e18db2956188d88715&wbrowser_core=1&mid=4895914160559000", "page_id": "2315893000000041:2778e8aab17435e18db2956188d88715", "short_url": "http://t.cn/A6NCr2qo", "long_url": "https://mp.weixin.qq.com/s/jD8qwmUbfwWBd97K0F0NMg", "url_type": 39, "result": true, "actionlog": { "act_type": 1, "act_code": 300, "oid": "3000000041:2778e8aab17435e18db2956188d88715", "uuid": 4895916153896975, "cardid": "", "lcardid": "", "uicode": "20000420", "luicode": "", "fid": "", "lfid": "", "ext": "mid:4895914160559000|rid:17_0_0_5117815272515082320_0_0_0|short_url:http://t.cn/A6NCr2qo|long_url:https://mp.weixin.qq.com/s/jD8qwmUbfwWBd97K0F0NMg|comment_id:|miduid:1670481425|rootmid:4895914160559000|rootuid:1670481425|authorid:|uuid:4895916153896975|is_ad_weibo:0|analysis_card:url_struct" }, "storage_type": "", "hide": 0, "object_type": "", "h5_target_url": "https://mp.weixin.qq.com/s/jD8qwmUbfwWBd97K0F0NMg", "need_save_obj": 0, "log": "su=A6NCr2qo&mark=&mid=4895914160559000" } ]”
 * user                     json 对象，每个 post 都包含
 * visible                  json 对象，示例：“{ "type": 0, "list_id": 0 }”，每个 post 都包含
 *
 * 代码中添加了 uid 字段，用于数据库中指向 user 表中的用户，添加了 retweeted_id 指向转发的 id。
 * 添加 created_at_timestamp 和 created_at_tz 字段，数据库中不直接存时间时区的字符串
 *
 * 部分 post 从网页接口拿不到，只能手机客户端和网页端能看到，这里先从网页端拿。网页端的字段略有不同，包含"ab_switcher", "ad_state"等字段，这里予以忽略，后面有必要删除不必要的字段，并增加 Repository 层以及 DTO 类型
 */
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
    pub retweeted_id: Option<i64>,
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
    pub retweeted_status: Option<Box<Post>>,
    #[sqlx(skip)]
    #[serde(default, deserialize_with = "deserialize_user")]
    pub user: Option<User>,
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
        let created_at = json["created_at"]
            .as_str()
            .map(parse_created_at)
            .ok_or(anyhow!("invalid created_at field"))??;
        let mut post: Post = from_value(json)?;
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

impl TryInto<Value> for Post {
    type Error = Error;
    fn try_into(self) -> Result<Value> {
        Ok(to_value(self)?)
    }
}

impl Post {
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

impl Post {
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

    pub async fn persist_posts(
        posts: Vec<Post>,
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
            Post::pic_ids_to_urls(pic_ids, pic_infos, image_definition)
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

    pub async fn generate_html<E>(
        posts: Vec<Post>,
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
        let gz = include_bytes!("../../../res/full.json.gz");
        let mut de = GzDecoder::new(gz.as_ref());
        let mut txt = String::new();
        de.read_to_string(&mut txt).unwrap();
        Ok(txt)
    }

    #[tokio::test]
    async fn create_table() {
        let db = create_db().await.unwrap();
        let mut conn = db.acquire().await.unwrap();
        Post::create_table(conn.as_mut()).await.unwrap();
    }

    #[test]
    fn deserialize_posts() {
        let test_case = load_test_case().unwrap();
        let test_case_val = serde_json::from_str::<Value>(&test_case).unwrap();
        let test_case_val_vec = serde_json::from_str::<Vec<Value>>(&test_case).unwrap();

        let _: Vec<Post> = serde_json::from_str(&test_case).unwrap();
        let _: Vec<Post> = serde_json::from_value(test_case_val).unwrap();
        let _: Vec<Post> = test_case_val_vec
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
            let _: Post = post
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
        Post::create_table(trans.as_mut()).await.unwrap();
        User::create_table(trans.as_mut()).await.unwrap();

        let test_case = serde_json::from_str::<Vec<Value>>(&load_test_case().unwrap())
            .unwrap()
            .into_iter()
            .map(|v| v.try_into().unwrap())
            .collect::<Vec<Post>>();
        for post in test_case {
            post.insert(trans.as_mut()).await.unwrap();
        }
        trans.commit().await.unwrap();
    }

    #[tokio::test]
    async fn query() {
        let ref db = create_db().await.unwrap();
        let mut trans = db.begin().await.unwrap();
        Post::create_table(trans.as_mut()).await.unwrap();
        User::create_table(trans.as_mut()).await.unwrap();

        let test_case = serde_json::from_str::<Vec<Value>>(&load_test_case().unwrap()).unwrap();
        let mut posts: HashMap<i64, Post> = HashMap::new();
        test_case.into_iter().for_each(|v| {
            let post: Post = v.try_into().unwrap();
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
            let mut post = Post::query(id, trans.as_mut()).await.unwrap().unwrap();
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
