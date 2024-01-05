use crate::{picture::Picture, post::Post, user::User};

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::{debug, info};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

const VALIDE_DB_VERSION: i64 = 1;
const DATABASE: &str = "res/weiback.db";

#[derive(Debug)]
pub struct Persister {
    db_path: PathBuf,
    db_pool: Option<SqlitePool>,
}

impl Persister {
    pub fn new() -> Self {
        Persister {
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
        Post::create_table(self.db().unwrap()).await?;
        User::create_table(self.db().unwrap()).await?;
        Picture::create_table(self.db().unwrap()).await?;
        Ok(())
    }

    pub fn db(&self) -> Option<&SqlitePool> {
        self.db_pool.as_ref()
    }
}
