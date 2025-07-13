use std::ops::DerefMut;

use anyhow::Result;
use serde_json::Value;
use sqlx::{Executor, FromRow, Sqlite};

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct UserStorage {
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
    pub icon_list: Option<Value>,
    #[sqlx(default)]
    pub backedup: bool,
}

impl UserStorage {
    pub async fn create_table<E>(mut db: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
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
        .execute(&mut *db)
        .await?;
        Ok(())
    }
}
