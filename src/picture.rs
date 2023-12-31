use crate::utils::pic_url_to_id;

use bytes::Bytes;
use sqlx::FromRow;

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

#[derive(Debug, Clone, FromRow)]
pub struct SqlPicture {
    pub id: String,
    pub uid: Option<i64>,
    pub post_id: Option<i64>,
    #[sqlx(rename = "type")]
    pub type_: u8,
}

impl From<&PictureMeta> for SqlPicture {
    fn from(value: &PictureMeta) -> Self {
        match value {
            PictureMeta::InPost(url, id) => Self {
                id: pic_url_to_id(url).into(),
                post_id: Some(*id),
                uid: None,
                type_: PIC_TYPE_INPOST,
            },
            PictureMeta::Avatar(url, id) => Self {
                id: pic_url_to_id(url).into(),
                post_id: None,
                uid: Some(*id),
                type_: PIC_TYPE_AVATAR,
            },
            PictureMeta::Emoji(url) => Self {
                id: pic_url_to_id(url).into(),
                post_id: None,
                uid: None,
                type_: PIC_TYPE_EMOJI,
            },
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SqlPictureBlob {
    pub url: String,
    pub id: String,
    pub blob: Vec<u8>,
}

const PIC_TYPE_AVATAR: u8 = 0;
const PIC_TYPE_INPOST: u8 = 1;
const PIC_TYPE_EMOJI: u8 = 2;

impl From<Picture> for SqlPictureBlob {
    fn from(value: Picture) -> Self {
        let url = value.meta.url();
        let id = pic_url_to_id(url).into();
        Self {
            url: url.into(),
            id,
            blob: value.blob.to_vec(),
        }
    }
}
