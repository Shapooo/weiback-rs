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

#[cfg(test)]
mod user_test {
    use super::UserInternal;
    use anyhow::Result;
    use flate2::read::GzDecoder;
    use serde_json::{from_str, Value};
    use std::io::Read;

    async fn load_test_case() -> Result<Vec<Value>> {
        let gz = include_bytes!("../../res/full.json.gz");
        let mut de = GzDecoder::new(gz.as_ref());
        let mut text = String::new();
        de.read_to_string(&mut text)?;

        let test_case_post: Vec<Value> = from_str(&text)?;
        let test_case = test_case_post
            .into_iter()
            .filter_map(|mut v| v["user"].is_object().then_some(v["user"].take()))
            .collect();
        Ok(test_case)
    }

    async fn parse_users(test_case: Vec<Value>) -> Result<Vec<UserInternal>> {
        test_case
            .into_iter()
            .map(|user| {
                let user: UserInternal = user.try_into()?;
                Ok(user)
            })
            .collect::<Result<Vec<_>>>()
    }

    #[tokio::test]
    async fn parse_from_json() {
        let test_case = load_test_case().await.unwrap();
        parse_users(test_case).await.unwrap();
    }
}
