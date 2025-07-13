use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PictureMeta {
    InPost { url: String, post_id: i64 },
    Avatar { url: String, user_id: i64 },
    Others { url: String },
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub meta: PictureMeta,
    pub blob: Bytes,
}
