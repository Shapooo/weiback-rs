use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::task_manager::TaskManager;
use crate::{api::ContainerType, config::Config, models::Post};

#[derive(Debug)]
pub struct TaskContext {
    pub task_id: Option<u64>,
    pub config: Config,
    pub task_manager: Arc<TaskManager>,
}

#[derive(Debug, Clone)]
pub enum TaskRequest {
    // to download favorites (range, with pic, image definition level)
    BackupFavorites(BackupFavoritesOptions),
    // to unfavorite favorite post
    UnfavoritePosts,
    // to backup user (id, with pic, image definition level)
    BackupUser(BackupUserPostsOptions),
    Export(ExportJobOptions),
}

impl TaskRequest {
    pub fn total(&self) -> u32 {
        match self {
            TaskRequest::BackupFavorites(options) => options.num_pages,
            TaskRequest::BackupUser(options) => options.num_pages,
            TaskRequest::UnfavoritePosts => 1,
            TaskRequest::Export(_) => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFavoritesOptions {
    pub num_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupUserPostsOptions {
    pub num_pages: u32,
    pub uid: i64,
    #[serde(default)]
    pub backup_type: BackupType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum BackupType {
    #[default]
    Normal,
    Original,
    Picture,
    Video,
    Article,
}

impl From<BackupType> for ContainerType {
    fn from(value: BackupType) -> Self {
        match value {
            BackupType::Normal => ContainerType::Normal,
            BackupType::Original => ContainerType::Original,
            BackupType::Picture => ContainerType::Picture,
            BackupType::Video => ContainerType::Video,
            BackupType::Article => ContainerType::Article,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum UserPostFilter {
    #[default]
    All,
    Original,
    Video,
    Picture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOutputConfig {
    pub task_name: String,
    pub export_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJobOptions {
    pub query: PostQuery,
    pub output: ExportOutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostQuery {
    pub user_id: Option<i64>,
    pub start_date: Option<i64>, // Unix timestamp
    pub end_date: Option<i64>,   // Unix timestamp
    pub is_favorited: bool,
    pub reverse_order: bool,
    // for pagination
    pub page: u32,
    pub posts_per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedPosts {
    pub posts: Vec<Post>,
    pub total_items: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInfo {
    pub post: Post,
    pub avatar_id: Option<String>,
    pub emoji_map: HashMap<String, String>,
    pub attachment_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedPostInfo {
    pub posts: Vec<PostInfo>,
    pub total_items: u64,
}
