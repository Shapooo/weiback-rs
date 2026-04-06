//! This module defines the internal `UserInternal` structure used for deserializing
//! user data directly from the Weibo API.
//!
//! It includes custom deserializers for various fields to handle inconsistencies
//! or specific formats in the API responses. It also provides a conversion
//! from `UserInternal` to the public `User` model.
use serde::Deserialize;
use serde_with::{NoneAsEmptyString, serde_as};
use url::Url;

use crate::models::User;

/// Internal representation of a Weibo user as received directly from the API.
///
/// This struct is used for deserialization and contains many optional fields
/// due to the varied nature of Weibo API responses. It includes custom deserializers
/// for specific data types.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UserInternal {
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub avatar_hd: Option<Url>,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub avatar_large: Option<Url>,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub following: bool,
    #[serde(default)]
    pub follow_me: bool,
    #[serde(default)]
    pub id: i64,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub profile_image_url: Option<Url>,
    #[serde(default)]
    pub screen_name: String,
}

impl From<UserInternal> for User {
    /// Converts an internal `UserInternal` structure into the public `User` model.
    ///
    /// This conversion assumes that essential URL fields like `avatar_hd`, `avatar_large`,
    /// and `profile_image_url` will always be present in a valid `UserInternal` and thus
    /// uses `expect`.
    ///
    /// # Arguments
    /// * `value` - The `UserInternal` to convert.
    ///
    /// # Returns
    /// A `User` model.
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
