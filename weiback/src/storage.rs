#![allow(async_fn_in_trait)]
pub mod database;
pub mod internal;
pub mod picture_storage;
pub mod video_storage;

use std::future::Future;
use std::ops::RangeInclusive;
use std::pin::Pin;

use bytes::Bytes;
use futures::{
    TryFutureExt,
    stream::{self, StreamExt},
};
use itertools::Itertools;
use log::{debug, error, info, warn};
use picture_storage::FileSystemPictureStorage;
use sqlx::SqlitePool;
use tokio::runtime::Runtime;
use url::Url;

use crate::core::task::{PaginatedPosts, PostQuery};
use crate::models::{Picture, Post, User, Video};
use crate::{
    error::{Error, Result},
    storage::video_storage::FileSystemVideoStorage,
};
use internal::post::{self, PostInternal};
use internal::user;

pub trait Storage: Send + Sync + Clone + 'static {
    async fn save_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, uid: i64) -> Result<Option<User>>;
    async fn get_favorites(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<Vec<Post>>;
    async fn get_posts(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<Vec<Post>>;
    async fn get_ones_posts(
        &self,
        uid: i64,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Vec<Post>>;
    async fn save_post(&self, post: &Post) -> Result<()>;
    async fn get_post(&self, id: i64) -> Result<Option<Post>>;
    async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPosts>;
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()>;
    async fn mark_post_favorited(&self, id: i64) -> Result<()>;
    async fn get_favorited_sum(&self) -> Result<u32>;
    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>>;
    fn save_picture(&self, picture: &Picture) -> impl Future<Output = Result<()>> + Send;
    async fn get_picture_blob(&self, url: &Url) -> Result<Option<bytes::Bytes>>;
    async fn picture_saved(&self, url: &Url) -> Result<bool>;
    fn save_video(&self, picture: &Video) -> impl Future<Output = Result<()>> + Send;
    async fn get_video_blob(&self, url: &Url) -> Result<Option<bytes::Bytes>>;
    async fn video_saved(&self, url: &Url) -> Result<bool>;
}

#[derive(Debug, Clone)]
pub struct StorageImpl {
    db_pool: SqlitePool,
    pic_storage: FileSystemPictureStorage,
    video_storage: FileSystemVideoStorage,
}

impl StorageImpl {
    pub fn new() -> Result<Self> {
        info!("Initializing storage...");
        let db_pool = Runtime::new()?
            .block_on(database::create_db_pool())
            .map_err(|e| {
                error!("Failed to create database pool: {e}");
                e
            })?;
        let pic_storage = FileSystemPictureStorage::new()?;
        let video_storage = FileSystemVideoStorage::new()?;

        info!("Storage initialized successfully.");
        Ok(StorageImpl {
            db_pool,
            pic_storage,
            video_storage,
        })
    }

    fn _save_post(&self, post: Post) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            debug!("Saving post with id: {}", post.id);
            if let Some(user) = &post.user {
                user::save_user(&self.db_pool, user).await?;
            }
            if let Some(ret_post) = post.retweeted_status.as_deref() {
                self._save_post(ret_post.clone()).await?;
            }
            let post_storage: PostInternal = post.try_into()?;
            match post::save_post(&self.db_pool, &post_storage, true).await {
                Ok(()) => {
                    debug!("Post with id: {} saved successfully", post_storage.id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to save post with id: {}: {:?}", post_storage.id, e);
                    Err(e)
                }
            }
        })
    }

    fn get_post(&self, id: i64) -> Pin<Box<dyn Future<Output = Result<Option<Post>>> + Send + '_>> {
        Box::pin(async move {
            if let Some(post) = post::get_post(&self.db_pool, id).await? {
                self.hydrate_post(post).await.map(Some)
            } else {
                Ok(None)
            }
        })
    }

    async fn hydrate_post(&self, post: PostInternal) -> Result<Post> {
        let user = if let Some(uid) = post.uid {
            user::get_user(&self.db_pool, uid).await?
        } else {
            None
        };

        let retweeted_status = if let Some(retweeted_id) = post.retweeted_id {
            Some(Box::new(self.get_post(retweeted_id).await?.ok_or(
                Error::DbError(format!(
                    "there's inconsistent data base status, cannot find post {}'s retweeted post {}",
                    post.id, retweeted_id
                )),
            )?))
        } else {
            None
        };
        let mut post: Post = post.try_into()?;
        post.retweeted_status = retweeted_status;
        post.user = user;
        Ok(post)
    }

    async fn hydrate_posts(&self, posts: Vec<PostInternal>) -> Vec<Post> {
        let posts = posts.into_iter().map(|p| {
            let id = p.id;
            self.hydrate_post(p).map_err(move |e| (id, e))
        });
        let (posts, errs): (Vec<_>, Vec<_>) = stream::iter(posts)
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .partition_result();

        warn!("{} posts constructed failed", errs.len());
        for (id, err) in errs {
            warn!("{id} cons failed: {err}");
        }
        posts
    }
}

impl Storage for StorageImpl {
    async fn get_favorites(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<Vec<Post>> {
        let (start, end) = range.into_inner();
        let posts = post::get_favorites(&self.db_pool, end - start + 1, start, reverse).await?;
        let posts = self.hydrate_posts(posts).await;
        debug!("geted {} favorites from local", posts.len());
        Ok(posts)
    }

    async fn get_posts(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<Vec<Post>> {
        let (start, end) = range.into_inner();
        let posts = post::get_posts(&self.db_pool, end - start + 1, start, reverse).await?;
        let posts = self.hydrate_posts(posts).await;
        debug!("geted {} post from local", posts.len());
        Ok(posts)
    }

    async fn get_ones_posts(
        &self,
        uid: i64,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Vec<Post>> {
        let (start, end) = range.into_inner();
        let posts =
            post::get_ones_posts(&self.db_pool, uid, end - start + 1, start, reverse).await?;
        let posts = self.hydrate_posts(posts).await;
        debug!("geted {} post of user {} from local", posts.len(), uid);
        Ok(posts)
    }

    async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        user::get_user(&self.db_pool, uid).await
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        self._save_post(post.clone()).await
    }

    async fn get_post(&self, id: i64) -> Result<Option<Post>> {
        self.get_post(id).await
    }

    async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPosts> {
        let (posts_internal, total_items) = post::query_posts(&self.db_pool, query).await?;
        let posts = self.hydrate_posts(posts_internal).await;
        Ok(PaginatedPosts { posts, total_items })
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        user::save_user(&self.db_pool, user).await
    }

    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        post::mark_post_unfavorited(&self.db_pool, id).await
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        post::mark_post_favorited(&self.db_pool, id).await
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        post::get_favorited_sum(&self.db_pool).await
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        post::get_posts_id_to_unfavorite(&self.db_pool).await
    }

    async fn get_picture_blob(&self, url: &Url) -> Result<Option<Bytes>> {
        self.pic_storage.get_picture_blob(&self.db_pool, url).await
    }

    async fn save_picture(&self, picture: &Picture) -> Result<()> {
        self.pic_storage.save_picture(&self.db_pool, picture).await
    }

    async fn picture_saved(&self, url: &Url) -> Result<bool> {
        self.pic_storage.picture_saved(&self.db_pool, url).await
    }

    async fn get_video_blob(&self, url: &Url) -> Result<Option<Bytes>> {
        self.video_storage.get_video_blob(&self.db_pool, url).await
    }

    async fn save_video(&self, video: &Video) -> Result<()> {
        self.video_storage.save_video(&self.db_pool, video).await
    }

    async fn video_saved(&self, url: &Url) -> Result<bool> {
        self.video_storage.video_saved(&self.db_pool, url).await
    }
}

#[cfg(test)]
mod local_tests {
    use std::{
        collections::{HashMap, HashSet},
        fs::read_to_string,
        path::Path,
    };

    use tempfile::tempdir;

    use super::*;
    use crate::{
        api::{favorites::FavoritesSucc, profile_statuses::ProfileStatusesSucc},
        models::Post,
    };

    async fn setup_storage() -> StorageImpl {
        let db_pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::migrate!().run(&db_pool).await.unwrap();
        let temp_dir = tempdir().unwrap();
        let pic_storage = FileSystemPictureStorage::from_picture_path(temp_dir.path().into());
        let video_storage = FileSystemVideoStorage::from_video_path(temp_dir.path().into());

        StorageImpl {
            db_pool,
            pic_storage,
            video_storage,
        }
    }

    async fn create_test_posts() -> Vec<Post> {
        let favorites = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/favorites.json");
        let s = read_to_string(favorites).unwrap();
        let favs = serde_json::from_str::<FavoritesSucc>(s.as_str()).unwrap();
        let mut favs: Vec<Post> = favs
            .favorites
            .into_iter()
            .map(|p| p.status.try_into())
            .collect::<Result<_>>()
            .unwrap();
        let profile_statuses =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/profile_statuses.json");
        let statuses = serde_json::from_str::<ProfileStatusesSucc>(
            read_to_string(profile_statuses).unwrap().as_str(),
        )
        .unwrap();
        let statuses: Vec<Post> = statuses
            .cards
            .into_iter()
            .filter_map(|c| c.mblog.map(|p| p.try_into()))
            .collect::<Result<_>>()
            .unwrap();
        favs.extend(statuses);
        favs
    }

    #[tokio::test]
    async fn test_save_and_get_favorites() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;

        let mut favorited_sum = 0;
        let posts = posts
            .into_iter()
            .map(|p| (p.id, p))
            .collect::<HashMap<i64, Post>>();
        for post in posts.values() {
            if post.favorited {
                favorited_sum += 1;
            }
            storage.save_post(post).await.unwrap();
        }

        let fetched_posts = storage
            .get_favorites(0..=1_000_000_000, false)
            .await
            .unwrap();

        assert_eq!(fetched_posts.len(), favorited_sum);

        for fetched in fetched_posts.iter() {
            let original = posts.get(&fetched.id).unwrap();
            if let (Some(original_user), Some(fetched_user)) = (&original.user, &fetched.user) {
                assert_eq!(original_user.id, fetched_user.id);
            }
            if let (Some(original_retweet), Some(fetched_retweet)) =
                (&original.retweeted_status, &fetched.retweeted_status)
            {
                assert_eq!(original_retweet.id, fetched_retweet.id);
                if let (Some(original_retweet_user), Some(fetched_retweet_user)) =
                    (&original_retweet.user, &fetched_retweet.user)
                {
                    assert_eq!(original_retweet_user.id, fetched_retweet_user.id);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_get_posts() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;
        let mut ids = HashSet::new();
        for post in posts.iter() {
            if let Some(ret) = post.retweeted_status.as_ref() {
                ids.insert(ret.id);
            }
            ids.insert(post.id);
            storage.save_post(post).await.unwrap();
        }

        let fetched_posts = storage.get_posts(0..=1000000, false).await.unwrap();
        assert_eq!(fetched_posts.len(), ids.len());

        let fetched_posts_rev = storage.get_posts(0..=1000000, true).await.unwrap();
        assert_eq!(fetched_posts_rev.len(), ids.len());
    }

    #[tokio::test]
    async fn test_get_ones_posts() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;
        let uid = 1786055427;
        let ones_posts_num = posts
            .iter()
            .filter(|p| {
                p.user.is_some() && p.user.as_ref().unwrap().id == uid
                    || p.retweeted_status.is_some()
                        && p.retweeted_status.as_ref().unwrap().id == uid
            })
            .count();

        for post in posts.iter() {
            storage.save_post(post).await.unwrap();
        }

        let fetched_posts = storage
            .get_ones_posts(uid, 0..=ones_posts_num as u32, false)
            .await
            .unwrap();
        assert_eq!(fetched_posts.len(), ones_posts_num);

        let fetched_posts_rev = storage
            .get_ones_posts(uid, 0..=ones_posts_num as u32, true)
            .await
            .unwrap();
        assert_eq!(fetched_posts_rev.len(), ones_posts_num);
    }

    #[tokio::test]
    async fn test_favorites_logic() {
        let storage = setup_storage().await;
        let posts = create_test_posts().await;

        let mut favorited = 0;
        let mut not_favorited = vec![];
        for post in posts {
            if post.favorited {
                favorited += 1;
            } else {
                not_favorited.push(post.id);
            }
            storage.save_post(&post).await.unwrap();
        }

        assert_eq!(storage.get_favorited_sum().await.unwrap(), favorited);

        let to_unfav = storage.get_posts_id_to_unfavorite().await.unwrap();
        assert_eq!(to_unfav.len(), favorited as usize);

        for id in to_unfav.iter().take(to_unfav.len() / 3) {
            storage.mark_post_unfavorited(*id).await.unwrap();
        }

        assert_eq!(
            storage.get_posts_id_to_unfavorite().await.unwrap().len() as u32,
            favorited - favorited / 3
        );

        for id in not_favorited.iter().take(to_unfav.len() / 3) {
            storage.mark_post_favorited(*id).await.unwrap();
        }

        assert_eq!(
            storage.get_posts_id_to_unfavorite().await.unwrap().len() as u32,
            favorited - favorited / 3 + not_favorited.len() as u32 / 3
        );
    }
}
