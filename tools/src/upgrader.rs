pub mod old_models;

use self::old_models::{get_old_posts, get_old_users};
use anyhow::Result;
use futures::stream::TryStreamExt;
use log::{info, warn};
use sqlx::SqlitePool;
use weiback::internals::storage_internal::{post::save_post, user::save_user};

pub struct Upgrader {
    old_db: SqlitePool,
    new_db: SqlitePool,
}

impl Upgrader {
    pub async fn new(old_db: SqlitePool, new_db: SqlitePool) -> Result<Self> {
        Ok(Self { old_db, new_db })
    }

    pub async fn migrate_all(&mut self, old_version: i64) -> Result<()> {
        info!("Migrating users...");
        self.migrate_users(old_version).await?;
        info!("Migrating posts and favorites...");
        self.migrate_posts_and_favorites(old_version).await?;
        // info!("Migrating pictures...");
        // self.migrate_pictures(old_version).await?;
        Ok(())
    }

    async fn migrate_users(&mut self, _old_version: i64) -> Result<()> {
        let mut old_users_stream = get_old_users(&self.old_db);

        while let Some(old_user) = old_users_stream.try_next().await? {
            match old_user.try_into() {
                Ok(user) => {
                    if let Err(e) = save_user(&self.new_db, &user).await {
                        warn!("Failed to save user {}: {}", user.id, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to convert old user record: {e}, skipping.");
                }
            }
        }
        info!("Users migration finished.");
        Ok(())
    }

    async fn migrate_posts_and_favorites(&mut self, old_version: i64) -> Result<()> {
        let mut old_posts_stream = get_old_posts(&self.old_db, old_version);

        while let Some(post) = old_posts_stream.try_next().await? {
            match post.try_into() {
                Ok(internal_post) => {
                    if let Err(e) = save_post(&self.new_db, &internal_post, true).await {
                        warn!("Failed to save post {}: {}", internal_post.id, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to convert old post record: {e}, skipping.");
                }
            }
        }

        info!("Posts and favorites migration finished.");
        Ok(())
    }
}
