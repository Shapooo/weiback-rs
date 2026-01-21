use log::{error, info};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::config::get_config;
use crate::error::{Error, Result};

pub async fn create_db_pool() -> Result<SqlitePool> {
    let db_path = get_config().read()?.db_path.clone();
    info!("Initializing database pool at path: {db_path:?}");
    let db_path = std::env::current_exe()?.parent().unwrap().join(db_path);
    if !db_path.exists() {
        info!("Database file not found at {db_path:?}. Creating new database...");
        if let Some(parent) = db_path.parent()
            && !parent.exists()
        {
            info!("Creating parent directory for database: {parent:?}");
            tokio::fs::create_dir_all(parent).await?;
        }

        Sqlite::create_database(db_path.to_str().unwrap()).await?;
        info!("Database file created.");
    }

    info!("Connecting to database and running migrations...");
    let db_pool = SqlitePool::connect(db_path.to_str().unwrap()).await?;

    sqlx::migrate!().run(&db_pool).await.map_err(|e| {
        error!("Database migration failed: {e}");
        Error::DbError(e.to_string())
    })?;

    info!("Database connection and migration successful.");
    Ok(db_pool)
}
