#![allow(unused)]
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LongText {
    #[serde(rename = "longTextContent")]
    pub long_text_content: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Status {
    #[serde(rename = "longText")]
    pub long_text: LongText,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BuildCommentsResponse {
    pub status: Status,
}
