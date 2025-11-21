use serde::Deserialize;
use serde_aux::prelude::*;
use url::Url;

use crate::models::User;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UserInternal {
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub avatar_hd: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub avatar_large: Option<Url>,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub following: bool,
    #[serde(default)]
    pub follow_me: bool,
    #[serde(default)]
    pub id: i64,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub profile_image_url: Option<Url>,
    #[serde(default)]
    pub screen_name: String,
}

impl From<UserInternal> for User {
    fn from(value: UserInternal) -> Self {
        Self {
            avatar_hd: value.avatar_hd.expect("promised to be Some"),
            avatar_large: value.avatar_large.expect("promised to be Some"),
            domain: value.domain,
            following: value.following,
            follow_me: value.follow_me,
            id: value.id,
            profile_image_url: value.profile_image_url.expect("promised to be Some"),
            screen_name: value.screen_name,
        }
    }
}
