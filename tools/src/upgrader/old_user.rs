use anyhow::{Context, Result};
use sqlx::SqlitePool;
use url::Url;

use weiback::models::User;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OldUser {
    pub id: i64,
    pub screen_name: Option<String>,
    pub profile_image_url: Option<String>,
    pub avatar_large: Option<String>,
    pub avatar_hd: Option<String>,
    pub domain: Option<String>,
    pub following: Option<bool>,
    pub follow_me: Option<bool>,
}

impl TryFrom<OldUser> for User {
    type Error = anyhow::Error;

    fn try_from(old: OldUser) -> Result<Self, Self::Error> {
        let id = old.id;

        let avatar_hd_url = old.avatar_hd.as_deref().and_then(|s| Url::parse(s).ok());
        let avatar_large_url = old.avatar_large.as_deref().and_then(|s| Url::parse(s).ok());
        let profile_image_url = old
            .profile_image_url
            .as_deref()
            .and_then(|s| Url::parse(s).ok());

        let hd = avatar_hd_url
            .clone()
            .or_else(|| avatar_large_url.clone())
            .or_else(|| profile_image_url.clone())
            .with_context(|| format!("user {id} has no valid avatar url for hd"))?;

        let large = avatar_large_url
            .clone()
            .or_else(|| avatar_hd_url.clone())
            .or_else(|| profile_image_url.clone())
            .with_context(|| format!("user {id} has no valid avatar url for large"))?;

        let profile = profile_image_url
            .or(avatar_large_url)
            .or(avatar_hd_url)
            .with_context(|| format!("user {id} has no valid profile image url"))?;

        Ok(User {
            id,
            screen_name: old.screen_name.unwrap_or_default(),
            avatar_hd: hd,
            avatar_large: large,
            profile_image_url: profile,
            domain: old.domain.unwrap_or_default(),
            following: old.following.unwrap_or(false),
            follow_me: old.follow_me.unwrap_or(false),
        })
    }
}

pub async fn get_old_users_paged(db: &SqlitePool, limit: i64, offset: i64) -> Result<Vec<OldUser>> {
    Ok(sqlx::query_as::<_, OldUser>(
        r#"SELECT
    id,
    screen_name,
    profile_image_url,
    avatar_large,
    avatar_hd,
    domain,
    following,
    follow_me
FROM
    users
ORDER BY
    id
LIMIT
    ?
OFFSET
    ?;"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?)
}

pub async fn get_users(db: &SqlitePool) -> Result<Vec<OldUser>> {
    Ok(sqlx::query_as("SELECT * FROM USERS").fetch_all(db).await?)
}
