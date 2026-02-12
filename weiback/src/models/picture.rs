use std::hash::Hash;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::Result;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PictureDefinition {
    Thumbnail,
    Bmiddle,
    Large,
    Original,
    Mw2000,
    #[default]
    Largest,
}

impl From<&str> for PictureDefinition {
    fn from(value: &str) -> Self {
        match value {
            "thumbnail" => PictureDefinition::Thumbnail,
            "bmiddle" => PictureDefinition::Bmiddle,
            "large" => PictureDefinition::Large,
            "original" => PictureDefinition::Original,
            "mw2000" => PictureDefinition::Mw2000,
            "largest" => PictureDefinition::Largest,
            _ => Self::default(), // Default case
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
            PictureDefinition::Mw2000 => "mw2000",
            PictureDefinition::Largest => "largest",
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub enum PictureMeta {
    Attached {
        url: Url,
        post_id: i64,
        definition: PictureDefinition,
    },
    Avatar {
        url: Url,
        user_id: i64,
    },
    Other {
        url: Url,
    },
}

impl Hash for PictureMeta {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url().hash(state);
    }
}

impl PartialEq for PictureMeta {
    fn eq(&self, other: &Self) -> bool {
        self.url() == other.url()
    }
}

impl PictureMeta {
    pub fn attached(url: &str, post_id: i64, definition: PictureDefinition) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(PictureMeta::Attached {
            url,
            definition,
            post_id,
        })
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
            PictureMeta::Attached { url, .. } => url,
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
