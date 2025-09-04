use serde::{Deserialize, Serialize};

/** 用户数据
 * 从微博 API 获取的每条 post 会附带 user 字段，原始数据为 Json 格式，包含如下字段：
 * avatar_hd            高清头像URL，字符串格式
 * avatar_large         大头像URL，字符格式
 * domain               字符串格式，示例：老胡该段为“huxijin”，有个梨GPT该段为“uglee”，更多的用户该段为空字符串
 * follow_me            布尔值，示例：true, false
 * following            布尔值，示例：true, false
 * icon_list            json 列表，示例：[ { "type": "vip", "data": { "mbrank": 7, "mbtype": 12, "svip": 0 } } ]，猜测为用户头像后的徽章，不作特别处理，保存时转为字符串存进数据库
 * id                   显然是用户ID
 * idstr                与上面的 id 字段重复，予以忽略
 * mbrank               整型，某种优先级，后续可考虑删除
 * mbtype               整形，某种分类，后续可考虑删除
 * pc_new               整形，后续可考虑删除
 * planet_video         布尔值，示例：true, false，与微博星球APP相关
 * profile_image_url    字符串，应该为用户详情背景图片
 * profile_url          字符串，用户链接后缀，示例："/u/1273725432"
 * screen_name          字符串，显示的用户名
 * v_plus               整型，通常为 0 或 1，应该是 v+ 会员标记
 * verified             布尔值，示例：true, false，是否认证
 * weihao               字符串，字符串内容通常为一串数字，用户个性化纯数字号码
 * verified_type        整型，通常是 0 或 -1
 * verified_type_ext    整形，通常为 0 或 1
 * vclub_member
 * 其中 vclub_member 和 verified_type_ext 不一定都会存在，其它字段都存在
 * 在上万份样本中只有两份出现了 vclub_member 且值都为1，所以忽略了该字段
 * 添加 backedup 字段，用于标识已经备份过的用户
 */
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct User {
    #[serde(default)]
    pub avatar_hd: String,
    #[serde(default)]
    pub avatar_large: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub following: bool,
    #[serde(default)]
    pub follow_me: bool,
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub profile_image_url: String,
    #[serde(default)]
    pub screen_name: String,
}
