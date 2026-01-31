use log::{error, info};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::config::get_config;
use crate::error::{Error, Result};

pub async fn create_db_pool() -> Result<SqlitePool> {
    let db_path = get_config().read()?.db_path.clone();
    info!("Initializing database pool at path: {db_path:?}");
    let db_path = std::env::current_exe()?.parent().unwrap().join(db_path);
    create_db_pool_with_url(db_path.to_str().unwrap()).await
}

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
