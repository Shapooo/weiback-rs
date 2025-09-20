use serde::{Deserialize, Serialize};
use url::Url;

use super::{PicInfoDetail, common::deserialize_nonable_url};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PicInfoItem {
    pub bmiddle: PicInfoDetail,
    pub large: PicInfoDetail,
    pub fid: Option<String>,
    pub focus_point: Option<FocusPoint>,
    pub largest: PicInfoDetail,
    pub mw2000: PicInfoDetail,
    pub original: PicInfoDetail,
    pub object_id: String,
    pub photo_tag: i32,
    pub pic_id: String,
    pub pic_status: i32,
    pub r#type: PicInfoType,
    pub thumbnail: PicInfoDetail,
    #[serde(default, deserialize_with = "deserialize_nonable_url")]
    pub video: Option<Url>,
    pub video_object_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nonable_url")]
    pub video_hd: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FocusPoint {
    pub height: f32,
    pub left: f32,
    pub top: f32,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PicInfoType {
    #[serde(rename = "pic")]
    Pic,
    #[serde(rename = "gif")]
    Gif,
    #[serde(rename = "livephoto")]
    Livephoto,
}

#[cfg(test)]
mod local_tests {
    use std::{fs::read_to_string, path::Path};

    use serde_json::{Value, from_str, from_value, to_value};

    use super::PicInfoItem;

    fn get_pic_infos() -> Vec<PicInfoItem> {
        let json_str =
            read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
                .unwrap();
        let mut value: Value = from_str(&json_str).unwrap();
        let pic_infos = value["favorites"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter_map(|item| {
                if let Some(ret) = item["status"]["retweeted_status"].as_object_mut() {
                    ret.remove("pic_infos")
                } else {
                    item["status"].as_object_mut().unwrap().remove("pic_infos")
                }
            })
            .collect::<Vec<_>>();
        pic_infos
            .into_iter()
            .flat_map(|v| {
                let res = if let Value::Object(v) = v {
                    Some(v.into_values())
                } else {
                    None
                };
                res.unwrap()
            })
            .map(|p| from_value::<PicInfoItem>(p).unwrap())
            .collect()
    }

    #[test]
    fn pic_info_item_conversion() {
        let pic_infos = get_pic_infos();
        for pic_info in pic_infos {
            let value = to_value(pic_info.clone()).unwrap();
            let new_pic_info = from_value(value).unwrap();
            assert_eq!(pic_info, new_pic_info);
        }
    }
}
