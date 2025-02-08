use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const USER_INFO_API: &str = "https://weibo.com/ajax/profile/info";

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct UserInternal {
    #[serde(default)]
    pub id: i64,
    pub profile_url: String,
    #[serde(default)]
    pub screen_name: String,
    #[serde(default)]
    pub profile_image_url: String,
    #[serde(default)]
    pub avatar_large: String,
    #[serde(default)]
    pub avatar_hd: String,
    #[serde(default)]
    pub planet_video: bool,
    #[serde(default, deserialize_with = "parse_v_plus")]
    pub v_plus: i64,
    #[serde(default)]
    pub pc_new: i64,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub verified_type: i64,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub weihao: String,
    pub verified_type_ext: Option<i64>,
    #[serde(default)]
    pub follow_me: bool,
    #[serde(default)]
    pub following: bool,
    #[serde(default)]
    pub mbrank: i64,
    #[serde(default)]
    pub mbtype: i64,
    pub icon_list: Option<Value>,
    #[serde(default)]
    pub backedup: bool,
}

pub fn parse_v_plus<'de, D>(deserializer: D) -> std::result::Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<i64>::deserialize(deserializer)?.unwrap_or_default())
}

impl TryFrom<Value> for UserInternal {
    type Error = Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        Ok(serde_json::from_value(value)?)
    }
}

impl TryInto<Value> for UserInternal {
    type Error = serde_json::Error;

    fn try_into(self) -> std::result::Result<Value, Self::Error> {
        serde_json::to_value(self)
    }
}
