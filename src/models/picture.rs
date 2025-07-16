use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PictureMeta {
    InPost { url: String, post_id: i64 },
    Avatar { url: String, user_id: i64 },
    Other { url: String },
}

impl PictureMeta {
    pub fn in_post(url: String, post_id: i64) -> Self {
        PictureMeta::InPost { url, post_id }
    }

    pub fn avatar(url: String, user_id: i64) -> Self {
        PictureMeta::Avatar { url, user_id }
    }

    pub fn other(url: String) -> Self {
        PictureMeta::Other { url }
    }
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub meta: PictureMeta,
    pub blob: Bytes,
}
