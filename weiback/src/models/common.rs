use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use url::Url;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PicInfoDetail {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub height: i32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub width: i32,
    pub url: Url,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HugeInfo {
    pub author_id: String,
    #[serde(default)]
    pub content1: String,
    #[serde(default)]
    pub content2: String,
    pub media_info: VideoInfo,
    pub object_id: String,
    pub object_type: String,
    pub oid: String,
    pub page_id: String,
    pub page_pic: String,
    pub page_title: String,
    pub page_url: Url,
    pub pic_info: PicInfoItemSimple,
    pub short_url: Url,
    pub r#type: String,
    pub type_icon: String,
    pub warn: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VideoInfo {
    pub author_mid: Option<String>,
    pub author_name: Option<String>,
    pub big_pic_info: Option<PicInfoItemSimple>,
    pub duration: Option<i32>,
    pub format: Option<String>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub h265_mp4_hd: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub h265_mp4_ld: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub h5_url: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub hevc_mp4_720p: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub inch_4_mp4_hd: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub inch_5_5_mp4_hd: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub inch_5_mp4_hd: Option<Url>,
    pub is_short_video: Option<i32>,
    pub kol_title: Option<String>,
    pub media_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub mp4_720p_mp4: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub mp4_hd_url: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub mp4_sd_url: Option<Url>,
    pub name: Option<String>,
    pub next_title: Option<String>,
    pub online_users: Option<String>,
    pub online_users_number: Option<i32>,
    pub origin_total_bitrate: Option<i32>,
    pub prefetch_size: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub stream_url: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub stream_url_hd: Option<Url>,
    #[serde(default, deserialize_with = "deserialize_to_type_or_none")]
    pub video_orientation: Option<Orientation>,
    pub video_publish_time: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PicInfoItemSimple {
    pub pic_big: PicInfoDetail,
    pub pic_middle: PicInfoDetail,
    pub pic_small: PicInfoDetail,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Orientation {
    #[serde(rename = "vertical")]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use std::{fs::read_to_string, path::Path};

    use serde_json::{Value, from_str, from_value, to_value};

    fn create_posts() -> Vec<Value> {
        let res =
            read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
                .unwrap();
        let mut value: Value = from_str(&res).unwrap();
        value["favorites"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .map(|f| f["status"].take())
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_huge_info_conversion() {
        let posts = create_posts();
        let mut count = 0;
        for mut post in posts {
            let mix_media_info = if let Some(v) = post["retweeted_status"].take().as_object_mut() {
                v.remove("mix_media_info")
            } else {
                post.as_object_mut().unwrap().remove("mix_media_info")
            };
            let Some(mut mmi) = mix_media_info else {
                continue;
            };
            if let Value::Array(mmi) = mmi["items"].take() {
                for v in mmi
                    .into_iter()
                    .filter(|m| m["data"]["content1"].is_string())
                    .map(|mut m| m["data"].take())
                {
                    count += 1;
                    let huge_info = from_value::<HugeInfo>(v).unwrap();
                    let v_huge_info = to_value(huge_info.clone()).unwrap();
                    let n_huge_info = from_value::<HugeInfo>(v_huge_info).unwrap();
                    assert_eq!(n_huge_info, huge_info);
                }
            }
        }
        assert!(count > 0);
    }

    #[test]
    fn test_video_info_conversion() {
        let posts = create_posts();
        let mut count = 0;
        for mut post in posts {
            let mix_media_info = if let Some(v) = post["retweeted_status"].take().as_object_mut() {
                v.remove("mix_media_info")
            } else {
                post.as_object_mut().unwrap().remove("mix_media_info")
            };
            let Some(mut mmi) = mix_media_info else {
                continue;
            };
            if let Value::Array(mmi) = mmi["items"].take() {
                for v in mmi
                    .into_iter()
                    .filter_map(|mut m| m["data"].as_object_mut().unwrap().remove("media_info"))
                {
                    count += 1;
                    let media_info = from_value::<VideoInfo>(v).unwrap();
                    let v_media_info = to_value(media_info.clone()).unwrap();
                    let n_media_info = from_value::<VideoInfo>(v_media_info).unwrap();
                    assert_eq!(n_media_info, media_info);
                }
            }
        }
        assert!(count > 0);
    }
}
