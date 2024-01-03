use crate::error::{Error, Result};

use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::{to_string, Value};
use sqlx::{FromRow, Sqlite, SqlitePool};

#[derive(Deserialize, Serialize, Debug, Clone, FromRow, PartialEq)]
pub struct User {
    pub id: i64,
    pub profile_url: String,
    pub screen_name: String,
    pub profile_image_url: String,
    pub avatar_large: String,
    pub avatar_hd: String,
    #[sqlx(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    pub v_plus: i64,
    #[sqlx(default)]
    pub pc_new: i64,
    #[sqlx(default)]
    pub verified: bool,
    #[sqlx(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    pub domain: String,
    #[sqlx(default)]
    pub weihao: String,
    #[sqlx(default)]
    pub verified_type_ext: Option<i64>,
    #[sqlx(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    pub following: bool,
    #[sqlx(default)]
    pub mbrank: i64,
    #[sqlx(default)]
    pub mbtype: i64,
    #[sqlx(json)]
    pub icon_list: Option<Value>,
    #[sqlx(default)]
    #[serde(default)]
    pub backedup: bool,
}

impl TryFrom<Value> for User {
    type Error = Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(|e| Error::Other(e.to_string()))
    }
}

impl TryInto<Value> for User {
    type Error = serde_json::Error;

    fn try_into(self) -> std::result::Result<Value, Self::Error> {
        serde_json::to_value(self)
    }
}

impl User {
    pub async fn create_table(db: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users ( \
             id INTEGER PRIMARY KEY, \
             profile_url TEXT, \
             screen_name TEXT, \
             profile_image_url TEXT, \
             avatar_large TEXT, \
             avatar_hd TEXT, \
             planet_video INTEGER, \
             v_plus INTEGER, \
             pc_new INTEGER, \
             verified INTEGER, \
             verified_type INTEGER, \
             domain TEXT, \
             weihao TEXT, \
             verified_type_ext INTEGER, \
             follow_me INTEGER, \
             following INTEGER, \
             mbrank INTEGER, \
             mbtype INTEGER, \
             icon_list TEXT, \
             backedup INTEGER \
             )",
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn insert(&self, db: &SqlitePool) -> Result<()> {
        debug!("insert user: {}", self.id);
        trace!("insert user: {:?}", self);
        let result = sqlx::query(
            "INSERT OR IGNORE INTO users VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(self.id)
        .bind(&self.profile_url)
        .bind(&self.screen_name)
        .bind(&self.profile_image_url)
        .bind(&self.avatar_large)
        .bind(&self.avatar_hd)
        .bind(self.planet_video)
        .bind(self.v_plus)
        .bind(self.pc_new)
        .bind(self.verified)
        .bind(self.verified_type)
        .bind(&self.domain)
        .bind(&self.weihao)
        .bind(self.verified_type_ext)
        .bind(self.follow_me)
        .bind(self.following)
        .bind(self.mbrank)
        .bind(self.mbtype)
        .bind(self.icon_list.as_ref().and_then(|v| to_string(&v).ok()))
        .bind(false)
        .execute(db)
        .await?;
        trace!("insert user {self:?}, result {result:?}");
        Ok(())
    }

    pub async fn mark_user_backed_up(&self, db: &SqlitePool) -> Result<()> {
        debug!("mark user {} backedup", self.id);
        sqlx::query("UPDATE users SET backedup = true WHERE id = ?")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn query_user(id: i64, db: &SqlitePool) -> Result<Self> {
        let user = sqlx::query_as::<Sqlite, User>(
            "SELECT id, profile_url, screen_name, profile_image_url, \
             avatar_large, avatar_hd, backedup FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(db)
        .await?;
        let user = user.ok_or(Error::Other(format!("user {} not found", id)))?;
        Ok(user)
    }
}
