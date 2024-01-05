use crate::error::{Error, Result};

use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::{to_string, Value};
use sqlx::{FromRow, Sqlite, SqlitePool};

#[derive(Deserialize, Serialize, Debug, Clone, FromRow, PartialEq)]
pub struct User {
    #[serde(default)]
    pub id: i64,
    pub profile_url: String,
    #[serde(default)]
    pub screen_name: String,
    #[serde(default)]
    pub profile_image_url: String,
    #[serde(default)]
    pub avatar_large: String,
    #[serde(default)]
    pub avatar_hd: String,
    #[sqlx(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    #[serde(deserialize_with = "parse_v_plus")]
    pub v_plus: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub pc_new: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub verified: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub verified_type: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub domain: String,
    #[sqlx(default)]
    #[serde(default)]
    pub weihao: String,
    #[sqlx(default)]
    pub verified_type_ext: Option<i64>,
    #[sqlx(default)]
    #[serde(default)]
    pub follow_me: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub following: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub mbrank: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub mbtype: i64,
    pub icon_list: Option<Value>,
    #[sqlx(default)]
    #[serde(default)]
    pub backedup: bool,
}

fn parse_v_plus<'de, D>(deserializer: D) -> std::result::Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<i64>::deserialize(deserializer)?.unwrap_or_default())
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

    pub async fn query(id: i64, db: &SqlitePool) -> Result<Option<Self>> {
        let user = sqlx::query_as::<Sqlite, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?;
        Ok(user)
    }
}

#[cfg(test)]
mod user_test {
    use super::User;
    use crate::error::Result;
    use flate2::read::GzDecoder;
    use futures::future::join_all;
    use serde_json::{from_str, Value};
    use std::collections::HashMap;
    use std::io::Read;

    async fn create_db() -> anyhow::Result<sqlx::SqlitePool> {
        Ok(sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await?)
    }

    async fn load_test_case() -> Result<Vec<Value>> {
        let gz = include_bytes!("../res/full.json.gz");
        let mut de = GzDecoder::new(gz.as_ref());
        let mut text = String::new();
        de.read_to_string(&mut text)?;

        let test_case_post: Vec<Value> = from_str(&text)?;
        let test_case = test_case_post
            .into_iter()
            .filter_map(|mut v| v["user"].is_object().then_some(v["user"].take()))
            .collect();
        Ok(test_case)
    }

    #[tokio::test]
    async fn create_table() {
        let db = create_db().await.unwrap();
        User::create_table(&db).await.unwrap();
    }

    async fn parse_users(test_case: Vec<Value>) -> Result<Vec<User>> {
        test_case
            .into_iter()
            .map(|user| {
                let user: User = user.try_into()?;
                Ok(user)
            })
            .collect::<Result<Vec<_>>>()
    }

    #[tokio::test]
    async fn parse_from_json() {
        let test_case = load_test_case().await.unwrap();
        parse_users(test_case).await.unwrap();
    }

    #[tokio::test]
    async fn insert() {
        let ref db = create_db().await.unwrap();
        User::create_table(&db).await.unwrap();
        let test_case = load_test_case().await.unwrap();
        let test_case = parse_users(test_case).await.unwrap();
        let test_case = test_case
            .into_iter()
            .filter(|user| user.id != i64::default())
            .collect::<Vec<_>>();
        join_all(
            test_case
                .into_iter()
                .map(|user| async move { user.insert(db).await }),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .unwrap();
    }

    #[tokio::test]
    async fn query() {
        let ref db = create_db().await.unwrap();
        User::create_table(db).await.unwrap();
        let test_case = load_test_case().await.unwrap();
        let test_case = parse_users(test_case).await.unwrap();
        let test_case = test_case
            .into_iter()
            .filter_map(|user| (user.id != i64::default()).then_some((user.id, user)))
            .collect::<HashMap<_, _>>();
        join_all(
            test_case
                .iter()
                .map(|(_, user)| async move { user.insert(db).await }),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .unwrap();
        let queried_user = join_all(
            test_case
                .iter()
                .map(|t| async move { User::query(*t.0, db).await }),
        )
        .await
        .into_iter()
        .filter_map(|user| user.transpose())
        .collect::<Result<Vec<_>>>()
        .unwrap();
        assert_eq!(queried_user.len(), test_case.len());
        queried_user.into_iter().for_each(|user| {
            let origin = test_case.get(&user.id).unwrap();
            assert_eq!(origin, &user);
        });
    }
}
