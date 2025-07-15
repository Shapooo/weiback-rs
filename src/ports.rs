#![allow(async_fn_in_trait)]
use std::sync::Arc;
use std::{ops::RangeInclusive, path::Path};

use egui::ImageData;
use weibosdk_rs::{Post, User};

use super::models::Picture;
use crate::error::{Error, Result};

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Task {
    // to fetch user meta data, include screen name and avatar
    FetchUserMeta(TaskOptions),
    // to download favorites (range, with pic, image definition level)
    BackupFavorites(TaskOptions),
    // to export favorites from local db (range, with pic, image definition level)
    ExportFromLocal(TaskOptions),
    // to unfavorite favorite post
    UnfavoritePosts,
    // to backup user (id, with pic, image definition level)
    BackupUser(TaskOptions),
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
    async fn unfavorite_posts(&self) -> Result<()>;
    async fn backup_favorites(&self, options: TaskOptions) -> Result<()>;
    async fn backup_user(&self, options: TaskOptions) -> Result<()>;
    async fn backup_self(&self, options: TaskOptions) -> Result<()>;
    async fn export_from_local(&self, options: TaskOptions) -> Result<()>;
}

pub trait Storage {
    async fn save_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, options: TaskOptions) -> Result<Option<User>>;
    async fn get_posts(&self, options: TaskOptions) -> Result<Vec<Post>>;
    async fn save_post(&self, post: &Post) -> Result<()>;
    async fn get_post(&self, options: TaskOptions) -> Result<Option<Post>>;
    async fn mark_post_unfavorited(&self, options: TaskOptions) -> Result<()>;
    async fn mark_post_favorited(&self, options: TaskOptions) -> Result<()>;
    async fn get_favorited_sum(&self) -> Result<u32>;
    async fn get_posts_id_to_unfavorite(&self) -> Result<Vec<i64>>;
    async fn save_picture(&self, picture: &Picture) -> Result<()>;
}

impl<S: Storage> Storage for Arc<S> {
    async fn save_user(&self, user: &User) -> Result<()> {
        self.as_ref().save_user(user).await
    }

    async fn get_user(&self, options: TaskOptions) -> Result<Option<User>> {
        self.as_ref().get_user(options).await
    }
    async fn get_post(&self, options: TaskOptions) -> Result<Option<Post>> {
        self.as_ref().get_post(options).await
    }

    async fn get_posts(&self, options: TaskOptions) -> Result<Vec<Post>> {
        self.as_ref().get_posts(options).await
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

    async fn mark_post_favorited(&self, options: TaskOptions) -> Result<()> {
        self.as_ref().mark_post_favorited(options).await
    }

    async fn mark_post_unfavorited(&self, options: TaskOptions) -> Result<()> {
        self.as_ref().mark_post_unfavorited(options).await
    }
    async fn save_picture(&self, picture: &Picture) -> Result<()> {
        self.as_ref().save_picture(picture).await
    }
}

pub trait Exporter: 'static + Clone + Send + Sync {
    async fn export_page(
        &self,
        task_name: &str,
        html: &str,
        target_dir: impl AsRef<Path>,
    ) -> Result<()>;
}

pub trait Processer: 'static + Clone + Send + Sync {
    async fn generate_html(&self) -> Result<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub enum UserPostFilter {
    #[default]
    All,
    Original,
    Video,
    Picture,
}

#[derive(Debug, Clone)]
pub struct TaskOptions {
    pub with_pic: bool,
    pub post_id: i64,
    pub uid: i64,
    pub pic_quality: PictureDefinition,
    pub reverse: bool,
    pub range: Option<RangeInclusive<u32>>,
    pub post_filter: UserPostFilter,
}

impl Default for TaskOptions {
    fn default() -> Self {
        Self {
            with_pic: false,
            post_id: 0,
            uid: 0,
            pic_quality: PictureDefinition::default(),
            reverse: false,
            range: None,
            post_filter: UserPostFilter::default(),
        }
    }
}

impl TaskOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pic(mut self) -> Self {
        self.with_pic = true;
        self
    }

    pub fn with_user(mut self, uid: i64) -> Self {
        self.uid = uid;
        self
    }

    pub fn with_post(mut self, post_id: i64) -> Self {
        self.post_id = post_id;
        self
    }

    pub fn pic_quality(mut self, quality: PictureDefinition) -> Self {
        self.pic_quality = quality;
        self
    }

    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub fn range(mut self, range: RangeInclusive<u32>) -> Self {
        self.range = Some(range);
        self
    }

    pub fn post_filter(mut self, filter: UserPostFilter) -> Self {
        self.post_filter = filter;
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum PictureDefinition {
    Thumbnail,
    Bmiddle,
    Large,
    Original,
    #[default]
    Largest,
    Mw2000,
}

impl From<PictureDefinition> for &str {
    fn from(value: PictureDefinition) -> Self {
        match value {
            PictureDefinition::Thumbnail => "thumbnail",
            PictureDefinition::Bmiddle => "bmiddle",
            PictureDefinition::Large => "large",
            PictureDefinition::Original => "original",
            PictureDefinition::Largest => "largest",
            PictureDefinition::Mw2000 => "mw2000",
        }
    }
}
