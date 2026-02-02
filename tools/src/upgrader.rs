pub mod old_picture;
pub mod old_post;
pub mod old_user;

use std::path::PathBuf;

use anyhow::Result;
use bytes::Bytes;
use log::{info, warn};
use sqlx::SqlitePool;
use url::Url;

use old_picture::get_old_pictures_paged;
use old_post::{convert_old_to_internal_post, get_old_posts_paged};
use old_user::get_old_users_paged;
use weiback::{
    internals::storage_internal::{post::save_post, user::save_user},
    models::{Picture, PictureDefinition, PictureMeta},
    storage::picture_storage::FileSystemPictureStorage,
};

fn get_definition_from_url(url_str: &str) -> Option<PictureDefinition> {
    let url = Url::parse(url_str).ok()?;
    let first_segment = url.path_segments()?.next()?;
    match first_segment {
        "wap180" => Some(PictureDefinition::Thumbnail),
        "wap360" => Some(PictureDefinition::Bmiddle),
        "orj960" => Some(PictureDefinition::Large),
        "orj1080" => Some(PictureDefinition::Original),
        "mw2000" => Some(PictureDefinition::Mw2000),
        "large" => Some(PictureDefinition::Largest),
        _ => None,
    }
}

pub struct Upgrader {
    old_db: SqlitePool,
    new_db: SqlitePool,
    pic_path: PathBuf,
}

impl Upgrader {
    pub async fn new(old_db: SqlitePool, new_db: SqlitePool, pic_path: PathBuf) -> Result<Self> {
        Ok(Self {
            old_db,
            new_db,
            pic_path,
        })
    }

    pub async fn migrate_all(&mut self, old_version: i64) -> Result<()> {
        info!("Migrating users...");
        self.migrate_users(old_version).await?;
        info!("Migrating posts and favorites...");
        self.migrate_posts_and_favorites(old_version).await?;
        info!("Migrating pictures...");
        self.migrate_pictures(old_version).await?;
        Ok(())
    }

    async fn migrate_users(&mut self, _old_version: i64) -> Result<()> {
        let mut tx = self.new_db.begin().await?;
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
                        if let Err(e) = save_user(&mut *tx, &user).await {
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
        tx.commit().await?;
        info!("Users migration finished.");
        Ok(())
    }

    async fn migrate_posts_and_favorites(&mut self, old_version: i64) -> Result<()> {
        let mut tx = self.new_db.begin().await?;
        let mut incompat_post_urls = Vec::new();
        let limit = 500;
        let mut offset = 0;
        loop {
            let old_posts = get_old_posts_paged(&self.old_db, old_version, limit, offset).await?;
            if old_posts.is_empty() {
                break;
            }

            for post in old_posts {
                match convert_old_to_internal_post(post, &mut incompat_post_urls) {
                    Ok(internal_post) => {
                        if let Err(e) = save_post(&mut *tx, &internal_post, true).await {
                            warn!("Failed to save post {}: {:?}", internal_post.id, e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to convert old post record: {e:?}, skipping.");
                    }
                }
            }
            offset += limit;
            info!("Processed {offset} posts...");
        }
        tx.commit().await?;

        warn!(
            "Some posts are INCOMPATIBLE for the conversion, we STRONGLY recommand you to RE-BACKUP:"
        );
        for url in incompat_post_urls {
            warn!("{url}");
        }

        info!("Posts and favorites migration finished.");
        Ok(())
    }

    async fn migrate_pictures(&self, _old_version: i64) -> Result<()> {
        let mut tx = self.new_db.begin().await?;
        let pic_storage = FileSystemPictureStorage;
        let limit = 500;
        let mut offset = 0;
        loop {
            let old_pictures = get_old_pictures_paged(&self.old_db, limit, offset).await?;
            if old_pictures.is_empty() {
                break;
            }

            for record in old_pictures {
                let url_str = record.url;
                let blob = record.blob;

                let meta = if let Some(user_id) = record.uid {
                    PictureMeta::avatar(&url_str, user_id)?
                } else if let Some(post_id) = record.post_id {
                    let definition = get_definition_from_url(&url_str).unwrap_or_else(|| {
                        warn!("cannot parse definition {url_str}");
                        PictureDefinition::Largest
                    });
                    PictureMeta::in_post(&url_str, definition, post_id)?
                } else {
                    PictureMeta::other(&url_str)?
                };

                let pic = Picture {
                    meta,
                    blob: Bytes::from(blob),
                };
                pic_storage
                    .save_picture(&self.pic_path, &mut *tx, &pic)
                    .await?;
            }

            offset += limit;
            info!("Processed {offset} pictures...");
        }
        tx.commit().await?;

        info!("Pictures migration finished");
        Ok(())
    }
}
