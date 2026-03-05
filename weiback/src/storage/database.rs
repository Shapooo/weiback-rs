//! This module handles the creation and management of the SQLite database connection pool,
//! including database file initialization and running migrations.
//!
//! It provides functions to create a database pool for both default application usage
//! and for custom database URLs, such as in-memory databases for testing.

use log::{error, info};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::config::get_config;
use crate::error::{Error, Result};

/// Creates a database connection pool using the default database path specified in the application configuration.
///
/// This function initializes the database file if it doesn't exist and runs all pending migrations.
///
/// # Returns
///
/// A `Result` containing a `SqlitePool` on success, or an `Error` on failure.
pub async fn create_db_pool() -> Result<SqlitePool> {
    let db_path = get_config().read()?.db_path.clone();
    info!("Initializing database pool at path: {db_path:?}");
    create_db_pool_with_url(db_path.to_str().unwrap()).await
}

/// Creates a database connection pool for a given database URL.
///
/// If the `db_url` is not `":memory:"`, it checks for the existence of the database file.
/// If the file does not exist, it creates it and any necessary parent directories.
/// It then connects to the database and runs all pending migrations.
///
/// # Arguments
///
/// * `db_url` - The URL of the SQLite database (e.g., `":memory:"` for an in-memory database, or a file path).
///
/// # Returns
///
/// A `Result` containing a `SqlitePool` on success, or an `Error` on failure.
pub async fn create_db_pool_with_url(db_url: &str) -> Result<SqlitePool> {
    if db_url != ":memory:" {
        let db_path = std::path::Path::new(db_url);
        if !db_path.exists() {
            info!("Database file not found at {db_path:?}. Creating new database...");
            if let Some(parent) = db_path.parent()
                && !parent.exists()
            {
                info!("Creating parent directory for database: {parent:?}");
                tokio::fs::create_dir_all(parent).await?;
            }

            Sqlite::create_database(db_url).await?;
            info!("Database file created.");
        }
    } else {
        info!("Initializing database pool in memory");
    }

    info!("Connecting to database and running migrations...");
    let db_pool = SqlitePool::connect(db_url).await?;

    sqlx::migrate!().run(&db_pool).await.map_err(|e| {
        error!("Database migration failed: {e}");
        Error::DbError(e.to_string())
    })?;

    info!("Database connection and migration successful.");
    Ok(db_pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_db_pool_with_url_memory() {
        let pool = create_db_pool_with_url(":memory:").await;
        assert!(pool.is_ok());
        let pool = pool.unwrap();
        // Verify that a table from migrations exists
        let res = sqlx::query_scalar::<Sqlite, i64>("SELECT COUNT(*) FROM posts")
            .fetch_one(&pool)
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_create_db_pool_with_url_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_db.sqlite");
        let db_url = db_path.to_str().unwrap();

        assert!(!db_path.exists());

        let pool = create_db_pool_with_url(db_url).await;
        assert!(pool.is_ok());
        assert!(db_path.exists());

        let pool = pool.unwrap();
        // Verify that a table from migrations exists
        let res = sqlx::query_scalar::<Sqlite, i64>("SELECT COUNT(*) FROM posts")
            .fetch_one(&pool)
            .await;
        assert!(res.is_ok());

        // Clean up
        dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_create_db_pool_with_url_existing_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("existing_db.sqlite");
        let db_url = db_path.to_str().unwrap();

        // Create the database file manually before calling the function
        Sqlite::create_database(db_url).await.unwrap();
        assert!(db_path.exists());

        let pool = create_db_pool_with_url(db_url).await;
        assert!(pool.is_ok());
        assert!(db_path.exists()); // Should still exist

        let pool = pool.unwrap();
        // Verify that a table from migrations exists
        let res = sqlx::query_scalar::<Sqlite, i64>("SELECT COUNT(*) FROM posts")
            .fetch_one(&pool)
            .await;
        assert!(res.is_ok());

        // Clean up
        dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_create_db_pool_with_url_non_existent_parent_dir() {
        let dir = tempdir().unwrap();
        let non_existent_parent = dir.path().join("non_existent_parent");
        let db_path = non_existent_parent.join("test_db.sqlite");
        let db_url = db_path.to_str().unwrap();

        assert!(!non_existent_parent.exists());
        assert!(!db_path.exists());

        let pool = create_db_pool_with_url(db_url).await;
        assert!(pool.is_ok());
        assert!(non_existent_parent.exists()); // Parent should have been created
        assert!(db_path.exists());

        let pool = pool.unwrap();
        // Verify that a table from migrations exists
        let res = sqlx::query_scalar::<Sqlite, i64>("SELECT COUNT(*) FROM posts")
            .fetch_one(&pool)
            .await;
        assert!(res.is_ok());

        // Clean up
        dir.close().unwrap();
    }
}
