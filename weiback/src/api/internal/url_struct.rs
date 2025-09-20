use std::borrow::Cow;
use std::collections::HashMap;
use std::result::Result;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use url::Url;

use crate::error::Error;
use crate::models::{PicInfosForStatusItem, UrlStruct, UrlStructItem, url_struct::UrlType};

#[derive(Debug, Clone, Deserialize)]
pub struct UrlStructInternal(pub Vec<UrlStructItemInternal>);

impl PartialEq for UrlStructInternal {
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UrlStructItemInternal {
    pub long_url: Option<String>,
    pub object_type: Option<String>,
    pub ori_url: String,
    pub page_id: Option<String>,
    pub short_url: String,
    pub url_title: String,
    pub url_type: UrlTypeInternal,
    pub url_type_pic: Option<String>,
    #[serde(default, deserialize_with = "deserialize_pic_ids")]
    pub pic_ids: Option<String>,
    #[serde(default, deserialize_with = "deserialize_pic_infos")]
    pub pic_infos: Option<PicInfosForStatusItem>,
    pub vip_gif: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrlTypeInternal {
    Link,
    Picture,
    Location,
    Appendix,
    Topic,
}

impl<'de> Deserialize<'de> for UrlTypeInternal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StrNum<'a> {
            Num(u8),
            Str(Cow<'a, str>),
        }
        match StrNum::deserialize(deserializer).unwrap() {
            StrNum::Num(0) => Ok(Self::Link),
            StrNum::Num(1) => Ok(Self::Picture),
            StrNum::Num(36) => Ok(Self::Location),
            StrNum::Num(39) => Ok(Self::Appendix),
            StrNum::Num(n) => {
                log::warn!("unknown url_struct type number {n}");
                Ok(Self::Link)
            }
            StrNum::Str(c) => {
                if c.is_empty() {
                    Ok(Self::Topic)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "unknown url_type str: {c}"
                    )))
                }
            }
        }
    }
}

impl From<UrlTypeInternal> for UrlType {
    fn from(value: UrlTypeInternal) -> Self {
        match value {
            UrlTypeInternal::Link => Self::Link,
            UrlTypeInternal::Picture => Self::Picture,
            UrlTypeInternal::Location => Self::Location,
            UrlTypeInternal::Appendix => Self::Appendix,
            UrlTypeInternal::Topic => Self::Topic,
        }
    }
}

impl TryFrom<UrlStructItemInternal> for UrlStructItem {
    type Error = Error;
    fn try_from(value: UrlStructItemInternal) -> std::result::Result<Self, Self::Error> {
        let res = Self {
            long_url: value.long_url.map(|url| Url::parse(&url)).transpose()?,
            object_type: value.object_type,
            ori_url: value.ori_url,
            page_id: value.page_id,
            short_url: Url::parse(&value.short_url)?,
            url_title: value.url_title,
            url_type: value.url_type.into(),
            url_type_pic: value.url_type_pic.map(|url| Url::parse(&url)).transpose()?,
            pic_ids: value.pic_ids,
            pic_infos: value.pic_infos,
            vip_gif: value.vip_gif,
        };
        Ok(res)
    }
}

impl TryFrom<UrlStructInternal> for UrlStruct {
    type Error = Error;
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

fn deserialize_pic_infos<'de, D>(deserializer: D) -> Result<Option<PicInfosForStatusItem>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MapOrItem {
        M(HashMap<String, PicInfosForStatusItem>),
        I(PicInfosForStatusItem),
    }
    let res = Option::<MapOrItem>::deserialize(deserializer)?.and_then(|mi| match mi {
        MapOrItem::I(i) => Some(i),
        MapOrItem::M(m) => {
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
