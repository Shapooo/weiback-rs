#![allow(async_fn_in_trait)]
mod post_storage;
mod processer;
mod user_storage;

use std::env::current_exe;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, error, info};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};
use tokio::runtime::Runtime;

use crate::config::get_config;
use crate::error::{Error, Result};
use crate::exporter::ExportOptions;
use crate::models::{Picture, Post, User};
use crate::utils::url_to_path;
use processer::Processer;

const VALIDE_DB_VERSION: i64 = 2;

pub trait Storage: Send + Sync + Clone + 'static {
    async fn save_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, uid: i64) -> Result<Option<User>>;
    async fn get_posts(&self, options: &ExportOptions) -> Result<Vec<Post>>;
    async fn save_post(&self, post: &Post) -> Result<()>;
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()>;
    async fn mark_post_favorited(&self, id: i64) -> Result<()>;
    async fn get_favorited_sum(&self) -> Result<u32>;
    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>>;
    fn save_picture(&self, picture: &Picture) -> impl Future<Output = Result<()>> + Send;
    async fn get_picture_blob(&self, url: &str) -> Result<Option<bytes::Bytes>>;
}

#[derive(Debug, Clone)]
pub struct StorageImpl {
    db_pool: SqlitePool,
    picture_path: PathBuf,
    processer: Processer,
}

impl StorageImpl {
    pub fn new() -> Result<Self> {
        info!("Initializing storage...");
        let config = get_config();
        let config_read = config.read().map_err(|e| {
            error!("Failed to read config lock: {e}");
            Error::Other(e.to_string())
        })?;
        let picture_path = config_read.picture_path.clone();
        drop(config_read);

        let db_pool = Runtime::new()
            .unwrap()
            .block_on(create_db_pool())
            .map_err(|e| {
                error!("Failed to create database pool: {e}");
                Error::Other(e.to_string())
            })?;

        let picture_path = current_exe().unwrap().parent().unwrap().join(picture_path);
        let processer = Processer::new(db_pool.clone());
        info!("Storage initialized successfully.");
        Ok(StorageImpl {
            processer,
            db_pool,
            picture_path,
        })
    }
}

impl Storage for Arc<StorageImpl> {
    async fn get_posts(&self, options: &ExportOptions) -> Result<Vec<Post>> {
        let start = options.range.start();
        let end = options.range.end();
        self.processer
            .get_posts(*end - *start + 1, *start, options.reverse)
            .await
    }

    async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        self.processer.get_user(uid).await
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        self.processer.save_post(post.clone()).await
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        self.processer.save_user(user).await
    }

    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        debug!("unfav post {id} in db");
        sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        debug!("mark favorited post {id} in db");
        sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
            .bind(id)
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        Ok(
            sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
                .fetch_one(&self.db_pool)
                .await?
                .0,
        )
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        debug!("query all posts to unfavorite");
        Ok(sqlx::query_as::<Sqlite, (i64,)>(
            "SELECT id FROM posts WHERE unfavorited == false and favorited;",
        )
        .fetch_all(&self.db_pool)
        .await?
        .into_iter()
        .map(|t| t.0)
        .collect())
    }

    // TODO: clarify semantic of Result and Option
    async fn get_picture_blob(&self, url: &str) -> Result<Option<Bytes>> {
        let path = url_to_path(url)?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap();
        let path = self.picture_path.join(relative_path);
        match tokio::fs::read(&path).await {
            Ok(blob) => Ok(Some(Bytes::from(blob))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn save_picture(&self, picture: &Picture) -> Result<()> {
        let path = url_to_path(picture.meta.url())?;
        let relative_path = Path::new(&path).strip_prefix("/").unwrap();
        let path = self.picture_path.join(relative_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &picture.blob).await?;
        debug!("picture {} saved to {:?}", picture.meta.url(), path);
        Ok(())
    }
}

async fn check_db_version(db_pool: &SqlitePool) -> Result<()> {
    let version = sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
        .fetch_one(db_pool)
        .await?;
    debug!("db version: {}", version.0);
    if version.0 == VALIDE_DB_VERSION {
        Ok(())
    } else {
        Err(Error::Other(
            "Invalid database version, please upgrade db file".to_string(),
        ))
    }
}

async fn create_db_pool() -> Result<SqlitePool> {
    let db_path = get_config()
        .read()
        .map_err(|e| Error::Other(e.to_string()))?
        .db_path
        .clone();
    info!("Initializing database pool at path: {db_path:?}");
    let db_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(db_path);
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

async fn create_tables(db_pool: &SqlitePool) -> Result<()> {
    let mut conn = db_pool.acquire().await?;
    post_storage::create_post_table(conn.as_mut()).await?;
    user_storage::create_user_table(conn.as_mut()).await?;
    sqlx::query(format!("PRAGMA user_version = {VALIDE_DB_VERSION};").as_str())
        .execute(db_pool)
        .await?;
    Ok(())
}
