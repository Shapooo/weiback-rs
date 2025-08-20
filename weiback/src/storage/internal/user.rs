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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::models::{Post, User};
    use sqlx::SqlitePool;
    use weibosdk_rs::{
        FavoritesAPI, ProfileStatusesAPI,
        mock::{MockAPI, MockClient},
    };

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_user_table(&pool).await.unwrap();
        pool
    }

    async fn create_test_users() -> Vec<User> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let client = MockClient::new();
        client
            .set_favorites_response_from_file(
                manifest_dir.join("tests/data/favorites.json").as_path(),
            )
            .unwrap();
        client
            .set_profile_statuses_response_from_file(
                manifest_dir
                    .join("tests/data/profile_statuses.json")
                    .as_path(),
            )
            .unwrap();
        let api = MockAPI::from_session(client, Default::default());
        let posts: Vec<Post> = api
            .favorites(1)
            .await
            .unwrap()
            .into_iter()
            .chain(api.profile_statuses(1786055427, 1).await.unwrap())
            .collect();
        posts
            .into_iter()
            .filter_map(|p| p.user)
            .collect::<Vec<User>>()
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
