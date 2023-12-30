use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Picture {
    pub meta: PictureMeta,
    pub blob: Bytes,
}

impl PictureMeta {
    pub fn url(&self) -> &str {
        match self {
            PictureMeta::InPost(url, _) => url,
            PictureMeta::Avatar(url, _) => url,
            PictureMeta::Emoji(url) => url,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PictureMeta {
    InPost(String, i64),
    Avatar(String, i64),
    Emoji(String),
}
