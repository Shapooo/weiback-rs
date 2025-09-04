use serde::{Deserialize, Serialize};

use super::HugeInfo;
use super::PicInfoItem;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MixMediaInfo {
    pub items: Vec<MixMediaInfoItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum MixMediaInfoItem {
    #[serde(rename = "pic")]
    Pic { id: String, data: Box<PicInfoItem> },
    #[serde(rename = "video")]
    Video { id: String, data: Box<HugeInfo> },
}

#[cfg(test)]
mod local_tests {
    use super::*;
    use std::fs::read_to_string;
    use std::path::Path;

    use serde_json::{Value, from_str, from_value, to_value};

    fn create_reponse_str() -> String {
        read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json"))
            .unwrap()
    }

    #[test]
    fn mix_media_info_conversion() {
        let res = create_reponse_str();
        let mut value: Value = from_str(&res).unwrap();
        let mmis = value["favorites"]
            .take()
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter_map(|p| {
                p["status"].as_object_mut().take().and_then(|m| {
                    if let Some(ret) = m.get_mut("retweeted_status") {
                        ret.as_object_mut().unwrap().remove("mix_media_info")
                    } else {
                        m.remove("mix_media_info")
                    }
                })
            })
            .collect::<Vec<_>>();
        for mmi in mmis {
            let mmi = from_value::<MixMediaInfo>(mmi).unwrap();
            let vmmi = to_value(mmi.clone()).unwrap();
            let n_mmi = from_value::<MixMediaInfo>(vmmi).unwrap();
            assert_eq!(n_mmi, mmi);
        }
    }
}
