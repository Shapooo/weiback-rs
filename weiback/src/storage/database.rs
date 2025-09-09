use log::{debug, error, info};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use super::VALIDE_DB_VERSION;
use super::internal::{post, user};
use crate::config::get_config;
use crate::error::{Error, Result};

pub async fn check_db_version(db_pool: &SqlitePool) -> Result<()> {
    let version = sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
        .fetch_one(db_pool)
        .await?;
    debug!("db version: {}", version.0);
    if version.0 == VALIDE_DB_VERSION {
        Ok(())
    } else {
        Err(Error::DbError(
            "Invalid database version, please upgrade db file".to_string(),
        ))
    }
}

pub async fn create_db_pool() -> Result<SqlitePool> {
    let db_path = get_config().read()?.db_path.clone();
    info!("Initializing database pool at path: {db_path:?}");
    let db_path = std::env::current_exe()?.parent().unwrap().join(db_path);
    if db_path.is_file() {
        info!("Database file exists at {db_path:?}. Connecting...");
        let db_pool = SqlitePool::connect(db_path.to_str().unwrap()).await?;
        check_db_version(&db_pool).await.map_err(|e| {
            error!("Database version check failed: {e}");
            e
        })?;
        info!("Database connection successful.");
        Ok(db_pool)
    } else {
        info!("Database file not found at {db_path:?}. Creating new database...");
        if let Some(parent) = db_path.parent()
            && !parent.exists()
        {
            info!("Creating parent directory for database: {parent:?}");
            tokio::fs::create_dir_all(parent).await?;
        }
        Sqlite::create_database(db_path.to_str().unwrap()).await?;
        info!("Database file created. Connecting...");
        let db_pool = SqlitePool::connect(db_path.to_str().unwrap()).await?;
        info!("Creating database tables...");
        create_tables(&db_pool).await?;
        info!("Database tables created successfully.");
        Ok(db_pool)
    }
}

pub async fn create_tables(db_pool: &SqlitePool) -> Result<()> {
    post::create_post_table(db_pool).await?;
    post::create_favorited_post_table(db_pool).await?;
    user::create_user_table(db_pool).await?;
    sqlx::query(format!("PRAGMA user_version = {VALIDE_DB_VERSION};").as_str())
        .execute(db_pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        SqlitePool::connect(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_create_tables() {
        let db = setup_db().await;
        let result = create_tables(&db).await;
        assert!(result.is_ok());

        // Verify that the tables were created
        let post_table_info = sqlx::query("PRAGMA table_info(posts);")
            .fetch_all(&db)
            .await;
        assert!(post_table_info.is_ok());
        assert!(!post_table_info.unwrap().is_empty());

        let user_table_info = sqlx::query("PRAGMA table_info(users);")
            .fetch_all(&db)
            .await;
        assert!(user_table_info.is_ok());
        assert!(!user_table_info.unwrap().is_empty());

        // Verify that the version was set
        let version = sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(version.0, VALIDE_DB_VERSION);
    }

    #[tokio::test]
    async fn test_check_db_version_ok() {
        let db = setup_db().await;
        create_tables(&db).await.unwrap();
        let result = check_db_version(&db).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_db_version_fail() {
        let db = setup_db().await;
        // Create tables but don't set version
        post::create_post_table(&db).await.unwrap();
        user::create_user_table(&db).await.unwrap();

        let result = check_db_version(&db).await;
        assert!(result.is_err());
    }
}
