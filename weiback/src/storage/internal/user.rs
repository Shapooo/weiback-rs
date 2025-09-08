use log::info;
use sqlx::{FromRow, Sqlite, SqlitePool};

use crate::error::Result;
use crate::models::User;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct UserInternal {
    pub avatar_hd: String,
    pub avatar_large: String,
    #[sqlx(default)]
    pub domain: String,
    pub following: bool,
    #[sqlx(default)]
    pub follow_me: bool,
    pub id: i64,
    pub profile_image_url: String,
    pub screen_name: String,
}

impl From<User> for UserInternal {
    fn from(value: User) -> Self {
        Self {
            avatar_hd: value.avatar_hd,
            avatar_large: value.avatar_large,
            domain: value.domain,
            following: value.following,
            follow_me: value.follow_me,
            id: value.id,
            profile_image_url: value.profile_image_url,
            screen_name: value.screen_name,
        }
    }
}

impl From<UserInternal> for User {
    fn from(val: UserInternal) -> Self {
        Self {
            avatar_hd: val.avatar_hd,
            avatar_large: val.avatar_large,
            domain: val.domain,
            following: val.following,
            follow_me: val.follow_me,
            id: val.id,
            profile_image_url: val.profile_image_url,
            screen_name: val.screen_name,
        }
    }
}

pub async fn create_user_table(db: &SqlitePool) -> Result<()> {
    info!("Creating user table if not exists...");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users ( \
             avatar_hd TEXT, \
             avatar_large TEXT, \
             domain TEXT, \
             following INTEGER, \
             follow_me INTEGER, \
             id INTEGER PRIMARY KEY, \
             profile_image_url TEXT, \
             screen_name TEXT \
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
        "INSERT OR REPLACE INTO users (\
             avatar_hd,\
             avatar_large,\
             domain,\
             following,\
             follow_me,\
             id,\
             profile_image_url,\
             screen_name)\
             VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user.avatar_hd)
    .bind(&user.avatar_large)
    .bind(&user.domain)
    .bind(user.following)
    .bind(user.follow_me)
    .bind(user.id)
    .bind(&user.profile_image_url)
    .bind(&user.screen_name)
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;
    use std::path::Path;

    use sqlx::SqlitePool;

    use super::*;
    use crate::api::{favorites::FavoritesSucc, profile_statuses::ProfileStatusesSucc};
    use crate::models::{Post, User};

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_user_table(&pool).await.unwrap();
        pool
    }

    async fn create_test_users() -> Vec<User> {
        let favorites = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json");
        let s = read_to_string(favorites).unwrap();
        let favs = serde_json::from_str::<FavoritesSucc>(s.as_str()).unwrap();
        let mut favs: Vec<Post> = favs
            .favorites
            .into_iter()
            .map(|p| p.status.into())
            .collect();
        let profile_statuses =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/profile_statuses.json");
        let statuses = serde_json::from_str::<ProfileStatusesSucc>(
            read_to_string(profile_statuses).unwrap().as_str(),
        )
        .unwrap();
        let statuses: Vec<Post> = statuses
            .cards
            .into_iter()
            .filter_map(|c| c.mblog.map(|p| p.into()))
            .collect();
        favs.extend(statuses);
        favs.into_iter().filter_map(|p| p.user).collect()
    }

    #[tokio::test]
    async fn test_user_conversion() {
        let users = create_test_users().await;
        for user in users {
            let internal_user: UserInternal = user.clone().into();
            let converted_user: User = internal_user.into();
            assert_eq!(user, converted_user);
        }
    }

    #[tokio::test]
    async fn test_create_user_table() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let result = create_user_table(&pool).await;
        assert!(result.is_ok());

        // Verify that the table was created by trying to insert a user
        let user = create_test_users().await.remove(0);
        let result = save_user(&pool, &user).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_and_get_user() {
        let db = setup_db().await;
        let users = create_test_users().await;
        for user in users {
            save_user(&db, &user).await.unwrap();
            let fetched_user = get_user(&db, user.id).await.unwrap().unwrap();
            assert_eq!(fetched_user, user);
        }
    }

    #[tokio::test]
    async fn test_get_non_existent_user() {
        let db = setup_db().await;
        let fetched_user = get_user(&db, 99999).await.unwrap();
        assert!(fetched_user.is_none());
    }

    #[tokio::test]
    async fn test_save_duplicate_user() {
        let db = setup_db().await;
        let users = create_test_users().await;
        for user in users {
            // Save the same user twice
            save_user(&db, &user).await.unwrap();
            let result = save_user(&db, &user).await;

            // Should not fail because of "INSERT OR IGNORE"
            assert!(result.is_ok());

            let fetched_user = get_user(&db, user.id).await.unwrap().unwrap();
            assert_eq!(fetched_user, user);
        }
    }
}
