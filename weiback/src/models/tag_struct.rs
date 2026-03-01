use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TagStruct(Vec<TagStructItem>);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TagStructItem {
    tag_name: String,
    url_type_pic: Url,
    otype: Option<String>,
    tag_hidden: Option<i32>,
    ori_url: Option<String>,
    desc: Option<String>,
}
