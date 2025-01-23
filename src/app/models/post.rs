use serde_json::Value;

use super::super::service::{emoticon::emoticon_get, search_args::SearchArgs};
use super::{picture::Picture, user::User};
use crate::exporter::{html_generator::HTMLGenerator, HTMLPage, HTMLPicture};

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
#[derive(Debug, Clone, PartialEq)]
pub struct Post {
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
    pub custom_icons: Option<Value>,
    pub number_display_strategy: Option<Value>,
    pub mix_media_info: Option<Value>,
    pub visible: Value,
    pub text: String,
    pub attitudes_status: i64,
    pub show_feed_repost: bool,
    pub show_feed_comment: bool,
    pub picture_viewer_sign: bool,
    pub show_picture_viewer: bool,
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
    pub text_length: Option<i64>,
    pub is_long_text: bool,
    pub rc_list: Option<Value>,
    pub annotations: Option<Value>,
    pub geo: Option<Value>,
    pub pic_focus_point: Option<Value>,
    pub page_info: Option<Value>,
    pub title: Option<Value>,
    pub continue_tag: Option<Value>,
    pub comment_manage_info: Option<Value>,
    pub client_only: bool,
    pub unfavorited: bool,
    pub created_at: String,
    pub created_at_timestamp: i64,
    pub created_at_tz: String,
    pub retweeted_status: Option<Box<Post>>,
    pub user: Option<User>,
}
