use serde::Deserialize;
use serde_aux::prelude::*;
use serde_json::Value;
use url::Url;

use crate::models::{
    common::VideoInfo,
    page_info::{PageInfo, PagePicInfo},
};

macro_rules! merge_optional_fields {
    ($target:expr, $source:expr, $($field:ident),+) => {
        $(
            if $target.$field.is_none() {
                $target.$field = $source.$field.take();
            }
        )+
    };
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PageInfoInternal {
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub author_id: Option<i64>,
    pub card_info: Option<Value>,
    pub cards: Option<Vec<PageInfoInternal>>,
    pub content1: Option<String>,
    pub content2: Option<String>,
    pub content3: Option<String>,
    pub content4: Option<String>,
    pub media_info: Option<VideoInfo>,
    pub object_id: Option<String>,
    pub object_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub oid: Option<i64>,
    pub page_desc: Option<String>,
    pub page_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub page_pic: Option<Url>,
    pub page_title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub page_url: Option<String>,
    pub pic_info: Option<PagePicInfo>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub short_url: Option<String>,
    pub source_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub r#type: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub type_icon: Option<Url>,
    pub user: Option<Value>,
}

impl From<PageInfoInternal> for PageInfo {
    fn from(mut page_info: PageInfoInternal) -> Self {
        if let Some(cards) = page_info.cards.take() {
            for mut card in cards {
                merge_optional_fields!(
                    page_info,
                    card,
                    author_id,
                    card_info,
                    content1,
                    content2,
                    content3,
                    content4,
                    media_info,
                    object_id,
                    object_type,
                    oid,
                    page_desc,
                    page_id,
                    page_pic,
                    page_title,
                    page_url,
                    pic_info,
                    short_url,
                    source_type,
                    r#type,
                    type_icon,
                    user
                );
            }
        }

        Self {
            author_id: page_info.author_id,
            card_info: page_info.card_info,
            content1: page_info.content1,
            content2: page_info.content2,
            content3: page_info.content3,
            content4: page_info.content4,
            media_info: page_info.media_info,
            object_id: page_info.object_id,
            object_type: page_info.object_type,
            oid: page_info.oid,
            page_desc: page_info.page_desc,
            page_id: page_info.page_id,
            page_pic: page_info.page_pic,
            page_title: page_info.page_title,
            page_url: page_info.page_url,
            pic_info: page_info.pic_info,
            short_url: page_info.short_url,
            source_type: page_info.source_type,
            r#type: page_info.r#type,
            type_icon: page_info.type_icon,
            user: page_info.user,
        }
    }
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use serde_json::{Value, from_str, from_value};
    use std::fs::read_to_string;
    use std::path::Path;

    fn create_response_str() -> String {
        read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
            .unwrap()
    }

    #[test]
    fn page_info_conversion() {
        let res = create_response_str();
        let mut value: Value = from_str(&res).unwrap();
        let posts = value["favorites"]
            .take()
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .map(|p| p["status"].take())
            .collect::<Vec<_>>();
        for mut post in posts {
            let Some(pi) = post.as_object_mut().and_then(|p| p.remove("page_info")) else {
                continue;
            };
            let _pi = from_value::<PageInfoInternal>(pi).unwrap();
        }
    }
}
