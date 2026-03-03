//! This module provides functions for interacting with the `users` table in the database.
//!
//! It handles the storage and retrieval of user metadata.
//!
//! # Table Structure: `users`
//!
//! | Column              | Type    | Description                                       |
//! |---------------------|---------|---------------------------------------------------|
//! | `avatar_hd`         | `TEXT`  | URL of the high-definition avatar.                |
//! | `avatar_large`      | `TEXT`  | URL of the large avatar.                          |
//! | `domain`            | `TEXT`  | User's custom domain (if any).                    |
//! | `following`         | `BOOLEAN` | Whether the current user is following this user.  |
//! | `follow_me`         | `BOOLEAN` | Whether this user is following the current user.  |
//! | `id`                | `INTEGER` | Unique identifier for the user. **Primary Key.** |
//! | `profile_image_url` | `TEXT`  | URL of the profile image.                         |
//! | `screen_name`       | `TEXT`  | User's screen name or nickname.                   |
//!
//! The `id` column serves as the primary key for uniqueness.

use sea_query::{Asterisk, Expr, OnConflict, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Executor, FromRow, Sqlite};
use url::Url;

use crate::error::{Error, Result};
use crate::models::User;

#[derive(sea_query::Iden)]
#[iden = "users"]
enum UserIden {
    Table,
    AvatarHd,
    AvatarLarge,
    Domain,
    Following,
    FollowMe,
    Id,
    ProfileImageUrl,
    ScreenName,
}

/// Represents the internal database structure for a user.
/// This struct is used for direct interaction with the `users` table.
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
    /// Converts a `User` model into a `UserInternal` database representation.
    fn from(value: User) -> Self {
        Self {
            avatar_hd: value.avatar_hd.to_string(),
            avatar_large: value.avatar_large.to_string(),
            domain: value.domain,
            following: value.following,
            follow_me: value.follow_me,
            id: value.id,
            profile_image_url: value.profile_image_url.to_string(),
            screen_name: value.screen_name,
        }
    }
}

impl TryFrom<UserInternal> for User {
    type Error = Error;
    /// Tries to convert a `UserInternal` database representation into a `User` model.
    /// This conversion can fail if URL strings are malformed.
    fn try_from(val: UserInternal) -> std::result::Result<Self, Self::Error> {
        let res = Self {
            avatar_hd: Url::parse(&val.avatar_hd)?,
            avatar_large: Url::parse(&val.avatar_large)?,
            domain: val.domain,
            following: val.following,
            follow_me: val.follow_me,
            id: val.id,
            profile_image_url: Url::parse(&val.profile_image_url)?,
            screen_name: val.screen_name,
        };
        Ok(res)
    }
}

/// Retrieves a single user by their ID.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `id` - The unique identifier of the user.
///
/// # Returns
///
/// A `Result` containing an `Option<User>`. `Some(User)` if the user is found, `None` otherwise.
pub async fn get_user<'e, E>(executor: E, id: i64) -> Result<Option<User>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::select()
        .column(Asterisk)
        .from(UserIden::Table)
        .and_where(Expr::col(UserIden::Id).eq(id))
        .build_sqlx(SqliteQueryBuilder);
    let user = sqlx::query_as_with::<_, UserInternal, _>(&sql, values)
        .fetch_optional(executor)
        .await?;
    user.map(|u| u.try_into()).transpose()
}

/// Saves a user's data into the database.
///
/// If a user with the same ID already exists, their data will be updated (UPSERT).
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `user` - The `User` object to save.
///
/// # Returns
///
/// A `Result` indicating success or failure.
pub async fn save_user<'e, E>(executor: E, user: &User) -> Result<()>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::insert()
        .into_table(UserIden::Table)
        .columns([
            UserIden::AvatarHd,
            UserIden::AvatarLarge,
            UserIden::Domain,
            UserIden::Following,
            UserIden::FollowMe,
            UserIden::Id,
            UserIden::ProfileImageUrl,
            UserIden::ScreenName,
        ])
        .values([
            user.avatar_hd.as_str().into(),
            user.avatar_large.as_str().into(),
            user.domain.as_str().into(),
            user.following.into(),
            user.follow_me.into(),
            user.id.into(),
            user.profile_image_url.as_str().into(),
            user.screen_name.as_str().into(),
        ])?
        .on_conflict(
            OnConflict::column(UserIden::Id)
                .update_columns([
                    UserIden::AvatarHd,
                    UserIden::AvatarLarge,
                    UserIden::Domain,
                    UserIden::Following,
                    UserIden::FollowMe,
                    UserIden::ProfileImageUrl,
                    UserIden::ScreenName,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);
    let _ = sqlx::query_with(&sql, values).execute(executor).await?;
    Ok(())
}

/// Retrieves a list of users by their IDs.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `ids` - A slice of user IDs.
///
/// # Returns
///
/// A `Result` containing a `Vec<User>` for the given IDs.
pub async fn get_users_by_ids<'e, E>(executor: E, ids: &[i64]) -> Result<Vec<User>>
where
    E: Executor<'e, Database = Sqlite>,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let (sql, values) = Query::select()
        .column(Asterisk)
        .from(UserIden::Table)
        .and_where(Expr::col(UserIden::Id).is_in(ids.iter().copied()))
        .build_sqlx(SqliteQueryBuilder);

    let records = sqlx::query_as_with::<_, UserInternal, _>(&sql, values)
        .fetch_all(executor)
        .await?;
    records.into_iter().map(|u| u.try_into()).collect()
}

/// Searches for users whose screen names start with a given prefix, limited to 20 results.
///
/// # Arguments
///
/// * `executor` - A database executor.
/// * `prefix` - The prefix to match against user screen names.
///
/// # Returns
///
/// A `Result` containing a `Vec<User>` matching the prefix.
pub async fn search_users_by_screen_name_prefix<'e, E>(
    executor: E,
    prefix: &str,
) -> Result<Vec<User>>
where
    E: Executor<'e, Database = Sqlite>,
{
    let (sql, values) = Query::select()
        .column(Asterisk)
        .from(UserIden::Table)
        .and_where(Expr::col(UserIden::ScreenName).like(format!("{}%", prefix)))
        .limit(20)
        .build_sqlx(SqliteQueryBuilder);
    let users: Vec<UserInternal> = sqlx::query_as_with(&sql, values)
        .fetch_all(executor)
        .await?;
    users.into_iter().map(|u| u.try_into()).collect()
}

#[cfg(test)]
mod local_tests {
    use std::path::Path;

    use sqlx::SqlitePool;
    use tokio::fs::read_to_string;

    use super::*;
    use crate::api::{favorites::FavoritesSucc, profile_statuses::ProfileStatusesSucc};
    use crate::error::Result;
    use crate::models::{Post, User};
    use crate::storage::database::create_db_pool_with_url;

    async fn setup_db() -> SqlitePool {
        create_db_pool_with_url(":memory:").await.unwrap()
    }

    async fn create_test_users() -> Vec<User> {
        let favorites = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json");
        let s = read_to_string(favorites).await.unwrap();
        let favs = serde_json::from_str::<FavoritesSucc>(s.as_str()).unwrap();
        let mut favs: Vec<Post> = favs
            .favorites
            .into_iter()
            .map(|p| p.status.try_into())
            .collect::<Result<_>>()
            .unwrap();
        let profile_statuses =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/profile_statuses.json");
        let statuses = serde_json::from_str::<ProfileStatusesSucc>(
            read_to_string(profile_statuses).await.unwrap().as_str(),
        )
        .unwrap();
        let statuses: Vec<Post> = statuses
            .cards
            .into_iter()
            .filter_map(|c| c.mblog.map(|p| p.try_into()))
            .collect::<Result<_>>()
            .unwrap();
        favs.extend(statuses);
        favs.into_iter().filter_map(|p| p.user).collect()
    }

    #[tokio::test]
    async fn test_user_conversion() {
        let users = create_test_users().await;
        for user in users {
            let internal_user: UserInternal = user.clone().into();
            let converted_user: User = internal_user.try_into().unwrap();
            assert_eq!(user, converted_user);
        }
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
