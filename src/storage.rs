use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use log::{debug, info, trace};
use serde_json::to_string;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

use super::app::models::{Picture, Post, User};
use crate::app::Storage;

const VALIDE_DB_VERSION: i64 = 2;
const DATABASE: &str = "res/weiback.db";

#[derive(Debug, Clone)]
pub struct StorageImpl {
    db_path: PathBuf,
    db_pool: Option<SqlitePool>,
}

impl StorageImpl {
    pub fn new() -> Self {
        StorageImpl {
            db_path: std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join(DATABASE),
            db_pool: None,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        debug!("initing...");
        if self.db_path.is_file() {
            info!("db {:?} exists", self.db_path);
            self.db_pool = Some(SqlitePool::connect(self.db_path.to_str().unwrap()).await?);
            self.check_db_version().await?;
        } else {
            info!("db {:?} not exists, create it", self.db_path);
            if !self.db_path.parent().unwrap().exists() {
                let mut dir_builder = tokio::fs::DirBuilder::new();
                dir_builder.recursive(true);
                dir_builder
                    .create(
                        self.db_path
                            .parent()
                            .ok_or(anyhow!("{:?} should have parent", self.db_path))?,
                    )
                    .await?;
            } else if self.db_path.parent().unwrap().is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "export folder is a already exist file",
                )
                .into());
            }
            Sqlite::create_database(self.db_path.to_str().unwrap()).await?;
            self.db_pool = Some(SqlitePool::connect(self.db_path.to_str().unwrap()).await?);
            self.create_db().await?;
        }
        Ok(())
    }

    async fn check_db_version(&mut self) -> Result<()> {
        let version = sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
            .fetch_one(self.db().unwrap())
            .await?;
        debug!("db version: {}", version.0);
        if version.0 == VALIDE_DB_VERSION {
            Ok(())
        } else {
            Err(anyhow!("Invalid database version, please upgrade db file"))
        }
    }

    async fn create_db(&mut self) -> Result<()> {
        let mut conn = self.db_pool.as_ref().unwrap().acquire().await?;
        Post::create_table(conn.as_mut()).await?;
        User::create_table(conn.as_mut()).await?;
        Picture::create_table(conn).await?;
        sqlx::query(format!("PRAGMA user_version = {};", VALIDE_DB_VERSION).as_str())
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    pub fn db(&self) -> Option<&SqlitePool> {
        self.db_pool.as_ref()
    }
}

impl Storage for Arc<StorageImpl> {
    async fn save_user(&mut self, user: User) -> Result<()> {
        debug!("insert user: {}", user.id);
        trace!("insert user: {:?}", user);
        let result = sqlx::query(
            "INSERT OR IGNORE INTO users (\
             id,\
             profile_url,\
             screen_name,\
             profile_image_url,\
             avatar_large,\
             avatar_hd,\
             planet_video,\
             v_plus,\
             pc_new,\
             verified,\
             verified_type,\
             domain,\
             weihao,\
             verified_type_ext,\
             follow_me,\
             following,\
             mbrank,\
             mbtype,\
             icon_list,\
             backedup)\
             VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id)
        .bind(&user.profile_url)
        .bind(&user.screen_name)
        .bind(&user.profile_image_url)
        .bind(&user.avatar_large)
        .bind(&user.avatar_hd)
        .bind(user.planet_video)
        .bind(user.v_plus)
        .bind(user.pc_new)
        .bind(user.verified)
        .bind(user.verified_type)
        .bind(&user.domain)
        .bind(&user.weihao)
        .bind(user.verified_type_ext)
        .bind(user.follow_me)
        .bind(user.following)
        .bind(user.mbrank)
        .bind(user.mbtype)
        .bind(user.icon_list.as_ref().and_then(|v| to_string(&v).ok()))
        .bind(false)
        .execute(self.db_pool.as_ref().unwrap())
        .await?;
        trace!("insert user {user:?}, result {result:?}");
        Ok(())
    }
}
