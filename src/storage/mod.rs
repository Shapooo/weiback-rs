mod post_storage;
mod user_storage;

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use log::{debug, info, trace};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::{
    models::{Picture, Post, User},
    ports::Storage,
};
use post_storage::PostStorage;
use user_storage::UserStorage;

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
        PostStorage::create_table(conn.as_mut()).await?;
        UserStorage::create_table(conn.as_mut()).await?;
        Picture::create_table(conn).await?;
        sqlx::query(format!("PRAGMA user_version = {};", VALIDE_DB_VERSION).as_str())
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    pub fn db(&self) -> Option<&SqlitePool> {
        self.db_pool.as_ref()
    }

    async fn load_complete_post(&self, post: &mut Post) -> Result<()> {
        if let Some(uid) = post.uid {
            post.user = self._get_user(uid).await?;
        }
        if let Some(retweeted_id) = post.retweeted_id {
            post.retweeted_status = Some(Box::new(self._get_post(retweeted_id).await?.ok_or(
                anyhow!(
                    "cannot find retweeted post {} of post {}",
                    retweeted_id,
                    post.id
                ),
            )?));
        }
        Ok(())
    }
}

impl Storage for StorageImpl {
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        debug!("unfav post {} in db", id);
        sqlx::query("UPDATE posts SET unfavorited = true WHERE id = ?")
            .bind(id)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        debug!("mark favorited post {} in db", id);
        sqlx::query("UPDATE posts SET favorited = true WHERE id = ?")
            .bind(id)
            .execute(self.db_pool.as_ref().unwrap())
            .await?;
        Ok(())
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        Ok(
            sqlx::query_as::<Sqlite, (u32,)>("SELECT COUNT(1) FROM posts WHERE favorited")
                .fetch_one(self.db_pool.as_ref().unwrap())
                .await?
                .0,
        )
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        debug!("query all posts to unfavorite");
        Ok(sqlx::query_as::<Sqlite, (i64,)>(
            "SELECT id FROM posts WHERE unfavorited == false and favorited;",
        )
        .fetch_all(self.db_pool.as_ref().unwrap())
        .await?
        .into_iter()
        .map(|t| t.0)
        .collect())
    }

    async fn get_post(&self, id: i64) -> Result<Option<Post>> {
        debug!("query post, id: {id}");
        if let Some(mut post) = self._get_post(id).await? {
            if let Some(retweeted_id) = post.retweeted_id {
                post.retweeted_status = Some(Box::new(self._get_post(retweeted_id).await?.ok_or(
                    anyhow!("cannot find retweeted post {} of post {}", retweeted_id, id),
                )?));
            }
            Ok(Some(post))
        } else {
            Ok(None)
        }
    }

    async fn get_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Vec<Post>> {
        debug!("query posts offset {offset}, limit {limit}, rev {reverse}");
        let sql_expr = if reverse {
            "SELECT * FROM posts WHERE favorited ORDER BY id LIMIT ? OFFSET ?"
        } else {
            "SELECT * FROM posts WHERE favorited ORDER BY id DESC LIMIT ? OFFSET ?"
        };
        let mut posts = sqlx::query_as::<sqlx::Sqlite, Post>(sql_expr)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db_pool.as_ref().unwrap())
            .await?;
        for post in posts.iter_mut() {
            self.load_complete_post(post).await?;
        }
        debug!("geted {} post from local", posts.len());
        Ok(posts)
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        debug!("insert post: {}", post.id);
        trace!("insert post: {:?}", post);
        self._save_post(post, true).await?;
        Ok(())
    }

    async fn get_user(&self, id: i64) -> Result<Option<User>> {
        self._get_user(id).await
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        self._save_user(&user).await
    }
}
