use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::models::PicInfoItem;

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
    pub url_type_pic: Option<Url>,
    pub pic_ids: Option<String>,
    pub pic_infos: Option<PicInfoItem>,
    pub vip_gif: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UrlType {
    Num(i32),
    Str(String),
}

impl Default for UrlType {
    fn default() -> Self {
        Self::Num(39)
    }
}

#[cfg(test)]
mod local_tests {
    use std::{fs::read_to_string, path::Path};

    use serde_json::{Value, from_str, from_value, to_value};

    use crate::api::internal::url_struct::UrlStructInternal;
    use crate::error::Result;
    use crate::models::UrlStruct;

    fn get_url_structs() -> Vec<UrlStruct> {
        let json_str =
            read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
                .unwrap();
        let mut value: Value = from_str(&json_str).unwrap();

        value["favorites"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter_map(|item| item["status"].get_mut("url_struct"))
            .map(|v| {
                from_value::<UrlStructInternal>(v.take())
                    .unwrap()
                    .try_into()
            })
            .collect::<Result<Vec<_>>>()
            .unwrap()
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
