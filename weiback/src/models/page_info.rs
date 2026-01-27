use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::models::PicInfoDetail;

use super::common::VideoInfo;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PageInfo {
    pub author_id: Option<i64>,
    pub card_info: Option<Value>,
    pub content1: Option<String>,
    pub content2: Option<String>,
    pub content3: Option<String>,
    pub content4: Option<String>,
    pub media_info: Option<VideoInfo>,
    pub object_id: Option<String>,
    pub object_type: Option<String>,
    pub oid: Option<i64>,
    pub page_desc: Option<String>,
    pub page_id: Option<String>,
    pub page_pic: Option<Url>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub pic_info: Option<PagePicInfo>,
    pub short_url: Option<String>,
    pub source_type: Option<String>,
    pub r#type: Option<i64>,
    pub type_icon: Option<Url>,
    pub user: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PagePicInfo {
    pub pic_big: PicInfoDetail,
}

#[cfg(test)]
mod local_tests {
    use std::fs::read_to_string;
    use std::path::Path;

    use serde_json::{Value, from_str, from_value, to_value};

    use super::*;
    use crate::api::internal::page_info::PageInfoInternal;

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
            let pi: PageInfo = from_value::<PageInfoInternal>(pi).unwrap().into();
            let vpi = to_value(pi.clone()).unwrap();
            let n_pi = from_value::<PageInfo>(vpi).unwrap();
            assert_eq!(n_pi, pi);
        }
    }
}
