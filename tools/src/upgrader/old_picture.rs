use anyhow::Result;
use sqlx::SqlitePool;
use url::Url;

use super::{old_post::OldPost, old_user::OldUser};
use weiback::utils::pic_url_to_id;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OldPictureBlob {
    pub url: String,
    #[allow(unused)]
    pub id: String,
    pub blob: Vec<u8>,
}

pub async fn get_pic_blob(db: &SqlitePool, id: &str) -> Result<Vec<OldPictureBlob>> {
    Ok(sqlx::query_as("SELECT * FROM picture_blob WHERE id = ?")
        .bind(id)
        .fetch_all(db)
        .await?)
}

pub fn extract_in_post_pic_ids(post: &OldPost) -> Result<Vec<String>> {
    let pic_ids: Vec<String> = if let Some(value) = &post.pic_ids {
        serde_json::from_value(value.clone())?
    } else {
        return Ok(Vec::new());
    };
    Ok(pic_ids)
}

fn pic_url_str_to_id(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|url| pic_url_to_id(&url).ok())
}

pub fn extract_avatar_id(user: &OldUser) -> Option<String> {
    user.avatar_hd
        .as_ref()
        .and_then(|url| pic_url_str_to_id(url))
        .or_else(|| {
            user.avatar_large
                .as_ref()
                .and_then(|url| pic_url_str_to_id(url))
        })
        .or_else(|| {
            user.profile_image_url
                .as_ref()
                .and_then(|url| pic_url_str_to_id(url))
        })
}
