use std::borrow::Cow;
use std::collections::HashMap;
use std::result::Result;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::PicInfoDetail;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UrlStruct(pub Vec<UrlStructItem>);

impl PartialEq for UrlStruct {
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UrlStructItem {
    pub long_url: Option<String>,
    pub object_type: Option<String>,
    pub ori_url: String,
    pub page_id: Option<String>,
    pub short_url: String,
    pub url_title: String,
    pub url_type: UrlType,
    pub url_type_pic: Option<String>,
    #[serde(default, deserialize_with = "deserialize_pic_ids")]
    pub pic_ids: Option<String>,
    #[serde(default, deserialize_with = "deserialize_pic_infos")]
    pub pic_infos: Option<PicInfosForStatusItem>,
    pub vip_gif: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PicInfosForStatusItem {
    pub bmiddle: PicInfoDetail,
    pub large: PicInfoDetail,
    pub thumbnail: PicInfoDetail,
    pub woriginal: PicInfoDetail,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum UrlType {
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "pic")]
    Picture,
    #[serde(rename = "loc")]
    Location,
    #[serde(rename = "appendix")]
    Appendix,
    #[serde(rename = "topic")]
    Topic,
}

impl<'de> Deserialize<'de> for UrlType {
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
                } else if c == "link" {
                    Ok(Self::Link)
                } else if c == "pic" {
                    Ok(Self::Picture)
                } else if c == "loc" {
                    Ok(Self::Location)
                } else if c == "appendix" {
                    Ok(Self::Appendix)
                } else if c == "topic" {
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

    use serde_json::{Value, from_str, from_value, to_value};

    use crate::models::UrlStruct;

    fn get_url_structs() -> Vec<UrlStruct> {
        let json_str =
            read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
                .unwrap();
        let mut value: Value = from_str(&json_str).unwrap();
        let url_structs = value["favorites"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter_map(|item| item["status"].get_mut("url_struct"))
            .map(|v| from_value(v.take()).unwrap())
            .collect::<Vec<_>>();
        url_structs
    }

    #[test]
    fn url_struct_conversion() {
        let url_structs = get_url_structs();
        for url_struct in url_structs {
            let value = to_value(url_struct.clone()).unwrap();
            let new_url_struct = from_value(value).unwrap();
            assert_eq!(url_struct, new_url_struct);
        }
    }
}
