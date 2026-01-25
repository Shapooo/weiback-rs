pub mod old_picture;
pub mod old_post;
pub mod old_user;

use std::collections::HashSet;

use anyhow::Result;
use bytes::Bytes;
use log::{info, warn};
use sqlx::SqlitePool;

use old_picture::{extract_avatar_id, extract_in_post_pic_ids, get_pic_blob};
use old_post::{convert_old_to_internal_post, get_old_posts_paged};
use old_user::{get_old_users_paged, get_users};
use weiback::{
    internals::storage_internal::{post::save_post, user::save_user},
    models::{Picture, PictureMeta},
    storage::picture_storage::FileSystemPictureStorage,
};

pub struct Upgrader {
    old_db: SqlitePool,
    new_db: SqlitePool,
    pic_storage: FileSystemPictureStorage,
}

impl Upgrader {
    pub async fn new(old_db: SqlitePool, new_db: SqlitePool) -> Result<Self> {
        let pic_storage = FileSystemPictureStorage::new()?;
        Ok(Self {
            old_db,
            new_db,
            pic_storage,
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
                        if let Err(e) = save_post(&self.new_db, &internal_post, true).await {
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

        warn!(
            "Some posts are INCOMPATIBLE for the conversion, we STRONGLY recommand you to RE-BACKUP:"
        );
        for url in incompat_post_urls {
            warn!("{url}");
        }

        info!("Posts and favorites migration finished.");
        Ok(())
    }

    async fn migrate_pictures(&self, old_version: i64) -> Result<()> {
        let mut pic_ids = HashSet::new();

        let limit = 500;
        let mut offset = 0;
        loop {
            let old_posts = get_old_posts_paged(&self.old_db, old_version, limit, offset).await?;
            if old_posts.is_empty() {
                break;
            }
            let mut page_pic_ids = Vec::new();
            for post in old_posts {
                let ids = extract_in_post_pic_ids(&post)?;
                for id in ids {
                    if pic_ids.insert(id.clone()) {
                        page_pic_ids.push((id, post.id));
                    }
                }
            }

            for (id, post_id) in page_pic_ids {
                if let Ok(pic_blobs) = get_pic_blob(&self.old_db, &id).await {
                    for pic_blob in pic_blobs {
                        let pic = Picture {
                            meta: PictureMeta::in_post(&pic_blob.url, post_id)?,
                            blob: Bytes::from(pic_blob.blob),
                        };
                        self.pic_storage.save_picture(&self.new_db, &pic).await?;
                    }
                }
            }

            offset += limit;
        }

        let users = get_users(&self.old_db).await?;
        let mut page_pic_ids = Vec::new();
        for user in users {
            if let Some(id) = extract_avatar_id(&user)
                && pic_ids.insert(id.clone())
            {
                page_pic_ids.push((id, user.id));
            }
        }
        for (id, user_id) in page_pic_ids {
            if let Ok(pic_blobs) = get_pic_blob(&self.old_db, &id).await {
                for pic_blob in pic_blobs {
                    let pic = Picture {
                        meta: PictureMeta::avatar(&pic_blob.url, user_id)?,
                        blob: Bytes::from(pic_blob.blob),
                    };
                    self.pic_storage.save_picture(&self.new_db, &pic).await?;
                }
            }
        }

        info!("Pictures migration finished");
        Ok(())
    }
}
