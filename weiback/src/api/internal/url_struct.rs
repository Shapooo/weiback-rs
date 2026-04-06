//! This module defines internal structures for deserializing `UrlStruct` data
//! from the Weibo API.
//!
//! It handles complex scenarios involving nested URL information, different
//! formats for picture IDs, and conversion into the public `UrlStruct` models.
use std::collections::HashMap;
use std::result::Result;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_with::{NoneAsEmptyString, serde_as};
use url::Url;

use crate::error::Error;
use crate::models::{PicInfoDetail, UrlStruct, UrlStructItem, url_struct::UrlType};
use crate::models::{PicInfoItem, PicInfoType};
use crate::utils::pic_url_to_id;

/// Internal representation of a collection of URL structures from the Weibo API.
///
/// This struct wraps a vector of `UrlStructItemInternal`, designed for direct deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct UrlStructInternal(pub Vec<UrlStructItemInternal>);

impl PartialEq for UrlStructInternal {
    /// Compares two `UrlStructInternal` instances for equality by comparing their inner vectors.
    fn eq(&self, other: &Self) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }

        for (l, r) in self.0.iter().zip(other.0.iter()) {
            if l != r {
                return false;
            }
        }
        true
    }
}

/// Internal representation of picture information for a status item, used during deserialization.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PicInfosForStatusItem {
    pub bmiddle: PicInfoDetail,
    pub large: PicInfoDetail,
    pub thumbnail: PicInfoDetail,
    pub woriginal: PicInfoDetail,
}

impl From<PicInfosForStatusItem> for PicInfoItem {
    /// Converts `PicInfosForStatusItem` (API specific) into the public `PicInfoItem` model.
    ///
    /// This conversion involves normalizing image URLs to derive various sizes
    /// (largest, mw2000, original) and determining the `PicInfoType` (Gif or Pic).
    ///
    /// # Arguments
    /// * `value` - The `PicInfosForStatusItem` to convert.
    ///
    /// # Returns
    /// A `PicInfoItem` with normalized picture details.
    fn from(value: PicInfosForStatusItem) -> Self {
        let mut url = value.bmiddle.url.clone();
        let large_path = url.path().replace("wap360", "large");
        let r#type = if large_path.ends_with("gif") {
            PicInfoType::Gif
        } else {
            PicInfoType::Pic
        };
        url.set_path(&large_path);
        let pic_id = pic_url_to_id(&url).unwrap_or_default();
        let largest = PicInfoDetail {
            height: 0,
            width: 0,
            url: url.clone(),
        };
        let mw2000_path = large_path.replace("large", "mw2000");
        url.set_path(&mw2000_path);
        let mw2000 = PicInfoDetail {
            height: 0,
            width: 0,
            url: url.clone(),
        };
        let original_path = large_path.replace("large", "orj1080");
        url.set_path(&original_path);
        let original = PicInfoDetail {
            height: 0,
            width: 0,
            url,
        };
        Self {
            bmiddle: value.bmiddle,
            large: value.large,
            fid: None,
            focus_point: None,
            largest,
            mw2000,
            original,
            object_id: None,
            photo_tag: 0,
            pic_id,
            pic_status: 1,
            r#type,
            thumbnail: value.thumbnail,
            video: None,
            video_object_id: None,
            video_hd: None,
        }
    }
}

/// Internal representation of a single item within a `UrlStruct` from the Weibo API.
///
/// This struct holds details about a URL, including its short and long forms, associated
/// page information, and custom deserializers for `pic_ids` and `pic_infos`.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UrlStructItemInternal {
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub long_url: Option<String>,
    pub object_type: Option<String>,
    pub ori_url: String,
    pub page_id: Option<String>,
    pub short_url: String,
    pub url_title: String,
    #[serde(default)]
    pub url_type: UrlType,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(default)]
    pub url_type_pic: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_pic_ids")]
    pub pic_ids: Option<String>,
    #[serde(default, deserialize_with = "deserialize_pic_infos")]
    pub pic_infos: Option<PicInfoItem>,
    pub vip_gif: Option<Value>,
}

impl TryFrom<UrlStructItemInternal> for UrlStructItem {
    type Error = Error;
    /// Converts an internal `UrlStructItemInternal` into the public `UrlStructItem` model.
    ///
    /// This conversion is straightforward, mapping fields directly.
    ///
    /// # Arguments
    /// * `value` - The `UrlStructItemInternal` to convert.
    ///
    /// # Returns
    /// A `Result` containing the `UrlStructItem` model.
    fn try_from(value: UrlStructItemInternal) -> std::result::Result<Self, Self::Error> {
        let res = Self {
            long_url: value.long_url,
            object_type: value.object_type,
            ori_url: value.ori_url,
            page_id: value.page_id,
            short_url: value.short_url,
            url_title: value.url_title,
            url_type: value.url_type,
            url_type_pic: value.url_type_pic,
            pic_ids: value.pic_ids,
            pic_infos: value.pic_infos,
            vip_gif: value.vip_gif,
        };
        Ok(res)
    }
}

impl TryFrom<UrlStructInternal> for UrlStruct {
    type Error = Error;
    /// Converts an internal `UrlStructInternal` into the public `UrlStruct` model.
    ///
    /// This involves converting each `UrlStructItemInternal` within the vector
    /// to its public `UrlStructItem` equivalent.
    ///
    /// # Arguments
    /// * `value` - The `UrlStructInternal` to convert.
    ///
    /// # Returns
    /// A `Result` containing the `UrlStruct` model.
    fn try_from(value: UrlStructInternal) -> Result<Self, Self::Error> {
        let res = Self(
            value
                .0
                .into_iter()
                .map(|u| u.try_into())
                .collect::<crate::error::Result<Vec<_>>>()?,
        );
        Ok(res)
    }
}

/// Custom deserializer for `pic_ids` that can handle either a single string or a vector of strings.
///
/// It extracts the first ID if a vector is provided, otherwise uses the single string.
/// Empty vectors result in `None`.
fn deserialize_pic_ids<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrVec {
        S(String),
        V(Vec<String>),
    }
    let res = Option::<StrOrVec>::deserialize(deserializer)?.and_then(|sv| match sv {
        StrOrVec::V(v) => {
            if v.is_empty() {
                None
            } else {
                v.into_iter().next()
            }
        }
        StrOrVec::S(s) => Some(s),
    });
    Ok(res)
}

/// Custom deserializer for `pic_infos` that can handle various JSON structures.
///
/// It attempts to deserialize into a `HashMap<String, PicInfosForStatusItem>`,
/// `HashMap<String, PicInfoItem>`, or a single `PicInfosForStatusItem`,
/// then converts the first item found into a `PicInfoItem`.
fn deserialize_pic_infos<'de, D>(deserializer: D) -> Result<Option<PicInfoItem>, D::Error>
where
    D: Deserializer<'de>,
{
    #[allow(clippy::large_enum_variant)]
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MapOrItem {
        M(HashMap<String, PicInfosForStatusItem>),
        M2(HashMap<String, PicInfoItem>),
        I(PicInfosForStatusItem),
    }
    let res = Option::<MapOrItem>::deserialize(deserializer)?.and_then(|mi| match mi {
        MapOrItem::I(i) => Some(i.into()),
        MapOrItem::M(m) => {
            if m.is_empty() {
                None
            } else {
                m.into_values().next().map(|i| i.into())
            }
        }
        MapOrItem::M2(m) => {
            if m.is_empty() {
                None
            } else {
                m.into_values().next()
            }
        }
    });
    Ok(res)
}

#[cfg(test)]
mod local_tests {
    use std::{fs::read_to_string, path::Path};

    use serde_json::{Value, from_str, from_value};

    use super::*;

    #[test]
    fn url_struct_conversion() {
        let json_str =
            read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
                .unwrap();
        let mut value: Value = from_str(&json_str).unwrap();
        let _url_structs = value["favorites"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter_map(|item| item["status"].get_mut("url_struct"))
            .map(|v| from_value::<UrlStructInternal>(v.take()).unwrap())
            .collect::<Vec<_>>();
    }
}
