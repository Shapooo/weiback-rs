use bytes::Bytes;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::Result;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum PictureDefinition {
    Thumbnail,
    Bmiddle,
    Large,
    Original,
    #[default]
    Largest,
    Mw2000,
}

impl From<&str> for PictureDefinition {
    fn from(value: &str) -> Self {
        match value {
            "thumbnail" => PictureDefinition::Thumbnail,
            "bmiddle" => PictureDefinition::Bmiddle,
            "large" => PictureDefinition::Large,
            "original" => PictureDefinition::Original,
            "largest" => PictureDefinition::Largest,
            "mw2000" => PictureDefinition::Mw2000,
            _ => PictureDefinition::Original, // Default case
        }
    }
}

impl From<&PictureDefinition> for &str {
    fn from(value: &PictureDefinition) -> Self {
        match value {
            PictureDefinition::Thumbnail => "thumbnail",
            PictureDefinition::Bmiddle => "bmiddle",
            PictureDefinition::Large => "large",
            PictureDefinition::Original => "original",
            PictureDefinition::Largest => "largest",
            PictureDefinition::Mw2000 => "mw2000",
        }
    }
}

impl From<u8> for PictureDefinition {
    fn from(value: u8) -> Self {
        match value {
            0 => PictureDefinition::Thumbnail,
            1 => PictureDefinition::Large,
            2.. => PictureDefinition::Original,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PictureMeta {
    InPost { url: Url, post_id: i64 },
    Avatar { url: Url, user_id: i64 },
    Other { url: Url },
}

impl PictureMeta {
    pub fn in_post(url: &str, post_id: i64) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(PictureMeta::InPost { url, post_id })
    }

    pub fn avatar(url: &str, user_id: i64) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(PictureMeta::Avatar { url, user_id })
    }

    pub fn other(url: &str) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(PictureMeta::Other { url })
    }

    pub fn url(&self) -> &Url {
        match self {
            PictureMeta::InPost { url, .. } => url,
            PictureMeta::Avatar { url, .. } => url,
            PictureMeta::Other { url } => url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub meta: PictureMeta,
    pub blob: Bytes,
}
