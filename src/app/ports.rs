use std::ops::RangeInclusive;

use anyhow::{Error, Result};
use egui::ImageData;

use super::models::{LongText, Picture, Post, User};

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

pub trait Storage: 'static + Clone + Send + Sync {
    async fn save_user(&self, user: User) -> Result<()>;
    async fn get_user(&self, id: i64) -> Result<Option<User>>;
    async fn mark_post_unfavorited(&self, id: i64) -> Result<()>;
    async fn mark_post_favorited(&self, id: i64) -> Result<()>;
    async fn get_post(&self, id: i64) -> Result<Option<Post>>;
}

pub trait Network: 'static + Clone + Send + Sync {
    async fn get_favorite_num(&self) -> Result<u32>;
    async fn get_user(&self, id: i64) -> Result<User>;
    async fn get_posts(
        &self,
        uid: i64,
        page: u32,
        search_args: &crate::app::service::search_args::SearchArgs,
    ) -> Result<Vec<Post>>;
    async fn unfavorite_post(&self, id: i64) -> Result<()>;
    async fn get_favorate_posts(&self, uid: i64, page: u32) -> Result<Vec<Post>>;
}

pub trait Exporter: 'static + Clone + Send + Sync {}
