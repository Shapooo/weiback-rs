mod upgrader;

use std::path::PathBuf;

use anyhow::Result;
use env_logger::Builder;
use log::{LevelFilter, error, info, warn};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};
use tokio::fs;

use upgrader::Upgrader;
use weiback::config;

#[tokio::main]
async fn main() {
    let res = start().await;
    if let Err(e) = res {
        error!("{e}");
    }
}

async fn start() -> Result<()> {
    init_logger()?;

    let old_db_path = PathBuf::from("weiback.db");
    let backup_db_path = PathBuf::from("weiback.bak.db");
    if old_db_path.exists() {
        info!("Found old database 'weiback.db', renaming to 'weiback.bak.db'.");
        fs::rename(&old_db_path, &backup_db_path).await?;
    } else if backup_db_path.exists() {
        info!("Using existing 'weiback.bak.db' for upgrade.");
    } else {
        info!(
            "Neither 'weiback.db' nor 'weiback.bak.db' found in current directory, nothing to upgrade."
        );
        return Ok(());
    }

    let backup_db_url = format!("sqlite:{}", backup_db_path.to_str().unwrap());
    let old_db_pool = SqlitePool::connect(&backup_db_url).await?;
    info!(
        "Successfully connected to backup database '{}'.",
        backup_db_path.display()
    );

    // Check if it's a new database
    let is_new = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_optional(&old_db_pool)
    .await?
    .is_some();

    if is_new {
        info!(
            "Database '{}' is already in the new format (contains _sqlx_migrations table), no upgrade needed.",
            backup_db_path.display()
        );
        return Ok(());
    }

    info!("Database is in old format, checking user_version...");
    let user_version = check_user_version(&old_db_pool).await?;
    info!("Old database user_version: {}", user_version);

    if user_version >= 3 {
        warn!(
            "Warn: the DB file has version {user_version}, which is not a supported version for upgrade, please download newest weiback-rs!"
        );
        return Ok(());
    } else if user_version < 0 {
        error!("Error: are you kidding? Invalid DB version");
        return Err(anyhow::anyhow!("Invalid DB version".to_string()));
    }

    config::init()?;
    info!("Config initialized.");

    let final_db_path = weiback::config::get_config()
        .read()
        .unwrap()
        .db_path
        .clone();
    let timestamp = chrono::Local::now().format("%Y%m%d").to_string();
    let temp_db_filename = format!(
        "{}-{}.db",
        final_db_path.file_stem().unwrap().to_str().unwrap(),
        timestamp
    );
    let temp_db_path = final_db_path.with_file_name(&temp_db_filename);

    info!(
        "Creating temporary new database pool at '{}'...",
        temp_db_path.display()
    );
    let new_db_pool = create_temp_db_pool(&temp_db_path).await?;
    info!("New database pool created and migrations run.");

    let mut upgrader = Upgrader::new(old_db_pool.clone(), new_db_pool.clone()).await?;

    match user_version {
        0..=2 => {
            info!("Upgrading from version {user_version}...");
            upgrader.migrate_all(user_version).await?;
        }
        _ => unreachable!(),
    };

    info!("Upgrade succeed!");
    old_db_pool.close().await;
    new_db_pool.close().await;

    if final_db_path.exists() {
        let current_time = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let backup_filename = format!(
            "{}-bak-{}.db",
            final_db_path.file_stem().unwrap().to_str().unwrap(),
            current_time
        );
        let backup_path = final_db_path.with_file_name(&backup_filename);
        warn!(
            "Final database file already exists at '{}'. Renaming it to '{}'.",
            final_db_path.display(),
            backup_path.display()
        );
        fs::rename(&final_db_path, &backup_path).await?;
    }
    fs::rename(&temp_db_path, &final_db_path).await?;
    info!(
        "Renamed temporary database to '{}'",
        final_db_path.display()
    );

    info!("Database upgrade completed successfully.");

    Ok(())
}

fn init_logger() -> Result<()> {
    let log_path = std::env::current_exe()?;
    let log_path = log_path
        .parent()
        .ok_or(anyhow::anyhow!(
            "the executable: {:?} should have parent, maybe bugs in there",
            std::env::current_exe()
        ))?
        .join("upgrade-db-tool.log");
    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(log_path)?;
    Builder::new()
        .filter_level(LevelFilter::Info)
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    Ok(())
}

async fn check_user_version(db: &SqlitePool) -> Result<i64> {
    Ok(sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
        .fetch_one(db)
        .await?
        .0)
}

async fn create_temp_db_pool(db_path: &PathBuf) -> Result<SqlitePool> {
    info!("Initializing temp database pool at path: {db_path:?}");

    if db_path.exists() {
        warn!("Temp database file already exists at {db_path:?}. Deleting it.");
        tokio::fs::remove_file(db_path).await?;
    }

    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        info!("Creating parent directory for database: {parent:?}");
        tokio::fs::create_dir_all(parent).await?;
    }

    sqlx::Sqlite::create_database(db_path.to_str().unwrap()).await?;
    info!("Temp database file created.");

    info!("Connecting to temp database and running migrations...");
    let db_pool = SqlitePool::connect(db_path.to_str().unwrap()).await?;

    sqlx::migrate!("../weiback/migrations")
        .run(&db_pool)
        .await?;

    info!("Database connection and migration successful.");
    Ok(db_pool)
}
