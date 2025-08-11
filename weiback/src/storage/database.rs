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
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                info!("Creating parent directory for database: {parent:?}");
                tokio::fs::create_dir_all(parent).await?;
            }
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
    user::create_user_table(db_pool).await?;
    sqlx::query(format!("PRAGMA user_version = {VALIDE_DB_VERSION};").as_str())
        .execute(db_pool)
        .await?;
    Ok(())
}
