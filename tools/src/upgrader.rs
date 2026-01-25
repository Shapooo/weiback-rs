pub mod old_post;
pub mod old_user;

use anyhow::Result;
use log::{info, warn};
use sqlx::SqlitePool;
use weiback::internals::storage_internal::{post::save_post, user::save_user};
use weiback::{
    internals::storage_internal::{post::save_post, user::save_user},
    models::{Picture, PictureMeta},
    storage::picture_storage::FileSystemPictureStorage,
};

use old_post::get_old_posts_paged;
use old_user::{get_old_users_paged, get_users};

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
        let limit = 500;
        let mut offset = 0;
        loop {
            let old_users = get_old_users_paged(&self.old_db, limit, offset).await?;
            if old_users.is_empty() {
                break;
            }

            for old_user in old_users {
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

            offset += limit;
            info!("Processed {offset} users...");
        }
        info!("Users migration finished.");
        Ok(())
    }

    async fn migrate_posts_and_favorites(&mut self, old_version: i64) -> Result<()> {
        let limit = 500;
        let mut offset = 0;
        loop {
            let old_posts = get_old_posts_paged(&self.old_db, old_version, limit, offset).await?;
            if old_posts.is_empty() {
                break;
            }

            for post in old_posts {
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
            offset += limit;
            info!("Processed {offset} posts...");
        }

        info!("Posts and favorites migration finished.");
        Ok(())
    }
}
