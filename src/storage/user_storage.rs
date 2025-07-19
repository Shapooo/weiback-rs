use std::ops::DerefMut;

use sqlx::{Executor, FromRow, Sqlite};

use crate::error::Result;
use crate::models::User;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct UserStorage {
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

impl From<User> for UserStorage {
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

impl Into<User> for UserStorage {
    fn into(self) -> User {
        User {
            id: self.id,
            screen_name: self.screen_name,
            profile_image_url: self.profile_image_url,
            avatar_large: self.avatar_large,
            avatar_hd: self.avatar_hd,
            verified: self.verified,
            verified_type: self.verified_type,
            domain: self.domain,
            follow_me: self.follow_me,
            following: self.following,
        }
    }
}

pub async fn create_user_table<E>(mut db: E) -> Result<()>
where
    E: DerefMut,
    for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
{
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
    .execute(&mut *db)
    .await?;
    Ok(())
}
