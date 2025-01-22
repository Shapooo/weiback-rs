use std::ops::RangeInclusive;
use std::sync::Arc;

use anyhow::{Error, Result};
use egui::ImageData;

use super::models::{LongText, Picture, Post, User};
use super::service::search_args::SearchArgs;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Task {
    // to fetch user meta data, include screen name and avatar
    FetchUserMeta(i64),
    // to download favorites (range, with pic, image definition level)
    BackupFavorites(RangeInclusive<u32>, bool, u8),
    // to export favorites from local db (range, with pic, image definition level)
    ExportFromLocal(RangeInclusive<u32>, bool, u8),
    // to unfavorite favorite post
    UnfavoritePosts,
    // to backup user (id, with pic, image definition level)
    BackupUser(i64, bool, u8),
}

#[allow(unused)]
pub enum TaskResponse {
    SumOfFavDB(u32, u32),             // remain sum of favorite in weibo and local db
    UserMeta(i64, String, ImageData), // screen name and avatar picture
    InProgress(f32, String),          // long time task is in progress
    Finished(u32, u32),               // long time task is finished
    Error(Error),                     // error occurs
}

pub trait Service {
    async fn unfavorite_posts(&self);
    async fn backup_favorites(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    );
    async fn backup_user(&self, uid: i64, with_pic: bool, image_definition: u8);
    async fn get_user_meta(&self, id: i64);
    async fn export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    );
}

pub trait Storage {
    async fn save_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, id: i64) -> Result<Option<User>>;
    async fn get_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Vec<Post>>;
    async fn save_post(&self, post: &Post) -> Result<()>;
    async fn get_post(&self, id: i64) -> Result<Option<Post>>;
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()>;
    async fn mark_post_favorited(&self, id: i64) -> Result<()>;
    async fn get_favorited_sum(&self) -> Result<u32>;
    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>>;
    async fn save_picture(&self, picture: &Picture) -> Result<()>;
}

impl<S: Storage> Storage for Arc<S> {
    async fn save_user(&self, user: &User) -> Result<()> {
        self.as_ref().save_user(user).await
    }

    async fn get_user(&self, id: i64) -> Result<Option<User>> {
        self.as_ref().get_user(id).await
    }
    async fn get_post(&self, id: i64) -> Result<Option<Post>> {
        self.as_ref().get_post(id).await
    }

    async fn get_posts(&self, limit: u32, offset: u32, reverse: bool) -> Result<Vec<Post>> {
        self.as_ref().get_posts(limit, offset, reverse).await
    }

    async fn save_post(&self, post: &Post) -> Result<()> {
        self.as_ref().save_post(post).await
    }

    async fn get_favorited_sum(&self) -> Result<u32> {
        self.as_ref().get_favorited_sum().await
    }

    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>> {
        self.as_ref().get_posts_id_to_unfavorite().await
    }

    async fn mark_post_favorited(&self, id: i64) -> Result<()> {
        self.as_ref().mark_post_favorited(id).await
    }

    async fn mark_post_unfavorited(&self, id: i64) -> Result<()> {
        self.as_ref().mark_post_unfavorited(id).await
    }
    async fn save_picture(&self, picture: &Picture) -> Result<()> {
        self.as_ref().save_picture(picture).await
    }
}

pub trait Network {
    async fn get_user(&self, id: i64) -> Result<User>;
    async fn get_favorite_num(&self) -> Result<u32>;
    async fn get_posts(&self, uid: i64, page: u32, search_args: &SearchArgs) -> Result<Vec<Post>>;
    async fn get_favorite_posts(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
    async fn unfavorite_post(&self, id: i64) -> Result<()>;
    async fn get_mobile_post(&self, mblogid: &str) -> Result<Post>;
    async fn get_long_text(&self, mblogid: &str) -> Result<Option<LongText>>;
}

pub trait Exporter: 'static + Clone + Send + Sync {}
