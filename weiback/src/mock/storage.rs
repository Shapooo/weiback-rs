//! Test mock for storage
use std::{
    collections::HashMap,
    future::Future,
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use bytes::Bytes;

use crate::{
    error::Result,
    models::{Picture, Post, User},
    storage::Storage,
};

#[derive(Debug, Clone, Default)]
pub struct StorageMock {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    users: HashMap<i64, User>,
    posts: HashMap<i64, (Post, bool)>,
    pictures: HashMap<String, Bytes>,
}

impl StorageMock {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Storage for StorageMock {
    async fn save_user(&self, user: &User) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.users.insert(user.id, user.clone());
        Ok(())
    }

    async fn get_user(&self, uid: i64) -> Result<Option<User>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.users.get(&uid).cloned())
    }

    async fn get_favorites(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<Vec<Post>> {
        let inner = self.inner.lock().unwrap();
        let mut posts: Vec<_> = inner
            .posts
            .values()
            .filter_map(|(p, _)| p.favorited.then(|| p))
            .cloned()
            .collect();
        posts.sort_by_key(|p| p.id);
        if reverse {
            posts.reverse();
        }
        let (start, end) = range.into_inner();
        let start = start as usize;
        let end = end as usize;
        if start >= posts.len() {
            return Ok(vec![]);
        }
        let end = std::cmp::min(end, posts.len() - 1);
        Ok(posts.get(start..=end).unwrap_or_default().to_vec())
    }

    async fn get_posts(&self, range: RangeInclusive<u32>, reverse: bool) -> Result<Vec<Post>> {
        let inner = self.inner.lock().unwrap();
        let mut posts: Vec<_> = inner.posts.values().map(|(p, _)| p).cloned().collect();
        posts.sort_by_key(|p| p.id);
        if reverse {
            posts.reverse();
        }
        let (start, end) = range.into_inner();
        let start = start as usize;
        let end = end as usize;
        if start >= posts.len() {
            return Ok(vec![]);
        }
        let end = std::cmp::min(end, posts.len() - 1);
        Ok(posts.get(start..=end).unwrap_or_default().to_vec())
    }

    async fn get_ones_posts(
        &self,
        uid: i64,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Vec<Post>> {
        let inner = self.inner.lock().unwrap();
        let mut posts: Vec<_> = inner
            .posts
            .values()
            .filter_map(|(p, _)| {
                p.user
                    .as_ref()
                    .and_then(|u| if u.id == uid { Some(p) } else { None })
            })
            .cloned()
            .collect();
        posts.sort_by_key(|p| p.id);
        if reverse {
            posts.reverse();
        }
        let (start, end) = range.into_inner();
        let start = start as usize;
        let end = end as usize;
        if start >= posts.len() {
            return Ok(vec![]);
        }
        let end = std::cmp::min(end, posts.len() - 1);
        Ok(posts.get(start..=end).unwrap_or_default().to_vec())
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.posts.insert(post.id, (post.clone(), false));
        Ok(())
    }

    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some((_, unfavorited)) = inner.posts.get_mut(&id) {
            *unfavorited = true;
        }
        Ok(())
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some((post, _)) = inner.posts.get_mut(&id) {
            post.favorited = true;
        }
        Ok(())
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.posts.values().filter(|(p, _)| p.favorited).count() as u32)
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .posts
            .values()
            .filter_map(|(p, unfavorited)| {
                if p.favorited && !*unfavorited {
                    Some(p.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }

    fn save_picture(&self, picture: &Picture) -> impl Future<Output = Result<()>> + Send {
        async move {
            self.inner
                .lock()
                .unwrap()
                .pictures
                .insert(picture.meta.url().to_string(), picture.blob.clone());
            Ok(())
        }
    }

    async fn get_picture_blob(&self, url: &str) -> Result<Option<bytes::Bytes>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.pictures.get(url).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use weibosdk_rs::{
        favorites::FavoritesAPI, mock_api::MockAPI, mock_client::MockClient,
        profile_statuses::ProfileStatusesAPI,
    };

    async fn create_posts() -> Vec<Post> {
        let client = MockClient::new();
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        client
            .set_favorites_response_from_file(
                manifest_dir.join("tests/data/favorites.json").as_path(),
            )
            .unwrap();
        client
            .set_profile_statuses_response_from_file(
                manifest_dir
                    .join("tests/data/profile_statuses.json")
                    .as_path(),
            )
            .unwrap();
        client
            .set_long_text_response_from_file(
                manifest_dir.join("tests/data/long_text.json").as_path(),
            )
            .unwrap();
        let api = MockAPI::from_session(client.clone(), Default::default());
        let mut posts = api.favorites(0).await.unwrap();
        posts.extend(api.profile_statuses(123, 0).await.unwrap());
        posts
    }

    #[tokio::test]
    async fn test_save_and_get_posts() {
        let storage = StorageMock::new();
        let posts = create_posts().await;
        for post in posts.iter() {
            storage.save_post(post).await.unwrap();
        }
        let fetched = storage.get_posts(0..=100, false).await.unwrap();
        assert_eq!(fetched.len(), posts.len());
    }

    #[tokio::test]
    async fn test_save_and_get_user() {
        let storage = StorageMock::new();
        let posts = create_posts().await;
        let users = posts.into_iter().filter_map(|p| p.user);
        for user in users {
            let id = user.id;
            storage.save_user(&user).await.unwrap();
            let fetched = storage.get_user(id).await.unwrap();
            assert_eq!(fetched.as_ref().unwrap().id, id);
        }
    }

    #[tokio::test]
    async fn test_favorites_logic() {
        let storage = StorageMock::new();
        let posts = create_posts().await;

        let mut favorated = 0;
        let mut not_favorited = vec![];
        for post in posts {
            if post.favorited {
                favorated += 1;
            } else {
                not_favorited.push(post.id);
            }
            storage.save_post(&post).await.unwrap();
        }

        assert_eq!(storage.get_favorited_sum().await.unwrap(), favorated);

        let to_unfav = storage.get_posts_id_to_unfavorite().await.unwrap();
        assert_eq!(to_unfav.len(), 20);

        for i in 0..to_unfav.len() / 3 {
            storage.mark_post_unfavorited(to_unfav[i]).await.unwrap();
        }

        assert_eq!(
            storage.get_posts_id_to_unfavorite().await.unwrap().len() as u32,
            favorated - favorated / 3
        );

        for i in 0..not_favorited.len() / 3 {
            storage.mark_post_favorited(not_favorited[i]).await.unwrap();
        }

        assert_eq!(
            storage.get_posts_id_to_unfavorite().await.unwrap().len() as u32,
            favorated - favorated / 3 + not_favorited.len() as u32 / 3
        );
    }

    #[tokio::test]
    async fn test_save_and_get_picture() {
        let storage = StorageMock::new();
        let picture = Picture {
            meta: crate::picture::PictureMeta::in_post("test_url".to_string(), 123),
            blob: Bytes::from_static(b"picture data"),
        };
        storage.save_picture(&picture).await.unwrap();
        let blob = storage.get_picture_blob(picture.meta.url()).await.unwrap();
        assert_eq!(blob.unwrap(), picture.blob);
    }
}
