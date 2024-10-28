use std::ops::DerefMut;

use anyhow::{Error, Result};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::{to_string, Value};
use sqlx::{Executor, FromRow, Sqlite};

use super::picture::Picture;

const USER_INFO_API: &str = "https://weibo.com/ajax/profile/info";

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
    #[serde(default)]
    pub planet_video: bool,
    #[sqlx(default)]
    #[serde(default, deserialize_with = "parse_v_plus")]
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
        Ok(serde_json::from_value(value)?)
    }
}

impl TryInto<Value> for User {
    type Error = serde_json::Error;

    fn try_into(self) -> std::result::Result<Value, Self::Error> {
        serde_json::to_value(self)
    }
}

impl User {
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

    // mark_user_backed_up must be called after all posts inserted,
    // to ensure the user info is persisted
    pub async fn mark_user_backed_up<E>(uid: i64, mut db: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        debug!("mark user {} backedup", uid);
        sqlx::query("UPDATE users SET backedup = true WHERE id = ?")
            .bind(uid)
            .execute(&mut *db)
            .await?;
        Ok(())
    }

    pub fn get_avatar_pic(&self, image_definition: u8) -> Picture {
        match image_definition {
            0 => Picture::avatar(self.profile_image_url.as_str(), self.id),
            1 => Picture::avatar(self.avatar_large.as_str(), self.id),
            2 => Picture::avatar(self.avatar_hd.as_str(), self.id),
            _ => unreachable!(),
        }
    }

    pub fn get_download_url(id: i64) -> String {
        format!("{}?uid={}", USER_INFO_API, id)
    }
}

#[cfg(test)]
mod user_test {
    use super::User;
    use anyhow::Result;
    use flate2::read::GzDecoder;
    use serde_json::{from_str, Value};
    use std::collections::HashMap;
    use std::io::Read;

    async fn create_db() -> Result<sqlx::SqlitePool> {
        Ok(sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await?)
    }

    async fn load_test_case() -> Result<Vec<Value>> {
        let gz = include_bytes!("../../../res/full.json.gz");
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
        let conn = db.acquire().await.unwrap();
        User::create_table(conn).await.unwrap();
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
        let db = create_db().await.unwrap();
        let mut trans = db.begin().await.unwrap();
        User::create_table(trans.as_mut()).await.unwrap();
        let test_case = load_test_case().await.unwrap();
        let test_case = parse_users(test_case).await.unwrap();
        let test_case = test_case
            .into_iter()
            .filter(|user| user.id != i64::default())
            .collect::<Vec<_>>();
        for user in test_case {
            user.insert(trans.as_mut()).await.unwrap();
        }
        trans.commit().await.unwrap();
    }

    #[tokio::test]
    async fn query() {
        let ref db = create_db().await.unwrap();
        let mut trans = db.begin().await.unwrap();
        User::create_table(trans.as_mut()).await.unwrap();
        let test_case = load_test_case().await.unwrap();
        let test_case = parse_users(test_case).await.unwrap();
        let test_case = test_case
            .into_iter()
            .filter_map(|user| (user.id != i64::default()).then_some((user.id, user)))
            .collect::<HashMap<_, _>>();
        for user in test_case.values() {
            user.insert(trans.as_mut()).await.unwrap();
        }
        let mut queried_user = Vec::new();
        for &id in test_case.keys() {
            queried_user.push(User::query(id, trans.as_mut()).await.unwrap());
        }
        let queried_user = queried_user
            .into_iter()
            .filter_map(|user| user)
            .collect::<Vec<_>>();
        assert_eq!(queried_user.len(), test_case.len());
        queried_user.into_iter().for_each(|user| {
            let origin = test_case.get(&user.id).unwrap();
            assert_eq!(origin, &user);
        });
    }
}
