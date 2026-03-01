use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{MixMediaInfo, PageInfo, PicInfoItem, TagStruct, UrlStruct, User};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Post {
    pub attitudes_count: Option<i64>,
    pub attitudes_status: i64,
    #[serde(with = "datetime")]
    pub created_at: DateTime<FixedOffset>,
    pub comments_count: Option<i64>,
    pub deleted: bool,
    pub edit_count: Option<i64>,
    pub favorited: bool,
    pub geo: Option<Value>,
    pub id: i64,
    pub mblogid: String,
    pub mix_media_ids: Option<Vec<String>>,
    pub mix_media_info: Option<MixMediaInfo>,
    pub page_info: Option<PageInfo>,
    pub pic_ids: Option<Vec<String>>,
    pub pic_infos: Option<HashMap<String, PicInfoItem>>,
    pub pic_num: Option<i64>,
    pub region_name: Option<String>,
    pub reposts_count: Option<i64>,
    pub repost_type: Option<i64>,
    pub retweeted_status: Option<Box<Post>>,
    pub source: Option<String>,
    pub tag_struct: Option<TagStruct>,
    pub text: String,
    pub url_struct: Option<UrlStruct>,
    pub user: Option<User>,
}

mod datetime {
    use std::borrow::Cow;

    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(dt: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&dt.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let created_at = Cow::<'_, str>::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&created_at).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod local_tests {
    use std::fs::read_to_string;
    use std::path::Path;

    use serde_json::from_str;

    use super::*;
    use crate::error::Result;

    fn create_reponse_str() -> String {
        read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
            .unwrap()
    }

    #[test]
    fn test_deserialize_post() {
        let response = create_reponse_str();
        let favorites = from_str::<crate::api::favorites::FavoritesSucc>(&response).unwrap();

        assert!(!favorites.favorites.is_empty());
    }

    #[test]
    fn test_post_serde_roundtrip() {
        let json_data = create_reponse_str();

        let parsed_favorites: crate::api::favorites::FavoritesSucc =
            serde_json::from_str(&json_data).expect("Failed to parse favorites.json");
        let posts = parsed_favorites
            .favorites
            .into_iter()
            .map(|f| f.status.try_into())
            .collect::<Result<Vec<Post>>>()
            .unwrap();

        for post in posts {
            let value_from_struct =
                serde_json::to_value(&post).expect("Failed to serialize Post to Value");

            let post_roundtrip: Post = serde_json::from_value(value_from_struct)
                .expect("Failed to deserialize Post from roundtrip Value");
            assert_eq!(post, post_roundtrip);
        }
    }
}
