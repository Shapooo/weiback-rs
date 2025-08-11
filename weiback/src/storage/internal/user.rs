use log::info;
use sqlx::{FromRow, Sqlite, SqlitePool};

use crate::error::Result;
use crate::models::User;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct UserInternal {
    pub id: i64,
    pub screen_name: String,
    pub profile_image_url: String,
    pub avatar_large: String,
    pub avatar_hd: String,
    #[sqlx(default)]
    pub verified: bool,
    #[sqlx(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    pub domain: String,
    #[sqlx(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    pub following: bool,
}

impl From<User> for UserInternal {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            screen_name: value.screen_name,
            profile_image_url: value.profile_image_url,
            avatar_large: value.avatar_large,
            avatar_hd: value.avatar_hd,
            verified: value.verified,
            verified_type: value.verified_type,
            domain: value.domain,
            follow_me: value.follow_me,
            following: value.following,
        }
    }
}

impl From<UserInternal> for User {
    fn from(val: UserInternal) -> Self {
        Self {
            id: val.id,
            screen_name: val.screen_name,
            profile_image_url: val.profile_image_url,
            avatar_large: val.avatar_large,
            avatar_hd: val.avatar_hd,
            verified: val.verified,
            verified_type: val.verified_type,
            domain: val.domain,
            follow_me: val.follow_me,
            following: val.following,
        }
    }
}

pub async fn create_user_table(db: &SqlitePool) -> Result<()> {
    info!("Creating user table if not exists...");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users ( \
             id INTEGER PRIMARY KEY, \
             screen_name TEXT, \
             profile_image_url TEXT, \
             avatar_large TEXT, \
             avatar_hd TEXT, \
             verified INTEGER, \
             verified_type INTEGER, \
             domain TEXT, \
             follow_me INTEGER, \
             following INTEGER \
             )",
    )
    .execute(db)
    .await?;
    info!("User table created successfully.");
    Ok(())
}

pub async fn get_user(db: &SqlitePool, id: i64) -> Result<Option<User>> {
    let user = sqlx::query_as::<Sqlite, UserInternal>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(user.map(|u| u.into()))
}

pub async fn save_user(db: &SqlitePool, user: &User) -> Result<()> {
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO users (\
             id,\
             screen_name,\
             profile_image_url,\
             avatar_large,\
             avatar_hd,\
             verified,\
             verified_type,\
             domain,\
             follow_me,\
             following)\
             VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(&user.screen_name)
    .bind(&user.profile_image_url)
    .bind(&user.avatar_large)
    .bind(&user.avatar_hd)
    .bind(user.verified)
    .bind(user.verified_type)
    .bind(&user.domain)
    .bind(user.follow_me)
    .bind(user.following)
    .bind(false)
    .execute(db)
    .await?;
    Ok(())
}
