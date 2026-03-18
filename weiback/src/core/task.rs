//! This module defines the data structures used to describe and configure tasks.
//!
//! It includes:
//! - [`TaskRequest`]: An enum representing the different types of operations (Backup, Export, Cleanup).
//! - [`TaskContext`]: Shared state and configuration passed throughout a task's execution.
//! - Various options structs (e.g., [`BackupUserPostsOptions`], [`ExportJobOptions`]).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::task_manager::TaskManager;
use crate::{api::ContainerType, config::Config, models::Post};

/// Context shared across a single task's execution.
#[derive(Debug)]
pub struct TaskContext {
    /// The unique ID of the task, if it is a long-running task.
    pub task_id: Option<u64>,
    /// A snapshot of the application configuration at task start.
    pub config: Config,
    /// Reference to the manager for progress and status reporting.
    pub task_manager: Arc<TaskManager>,
}

/// Represents a request to perform a specific application task.
#[derive(Debug, Clone)]
pub enum TaskRequest {
    /// Backup favorited posts from the currently logged-in user.
    BackupFavorites(BackupFavoritesOptions),
    /// Unfavorite posts that are currently in Weibo's favorites list.
    UnfavoritePosts,
    /// Backup posts from a specific user.
    BackupUser(BackupUserPostsOptions),
    /// Export saved posts from local storage to an external format (e.g., HTML).
    Export(ExportJobOptions),
    /// Clean up redundant images or enforce resolution policies.
    CleanupPictures(CleanupPicturesOptions),
    /// Clean up invalid or outdated user avatars.
    CleanupAvatars,
    /// Clean up invalid posts (e.g., user is None).
    CleanupInvalidPosts(CleanupInvalidPostsOptions),
    /// Re-backup posts based on a query.
    RebackupPosts(PostQuery),
    /// Re-backup posts that have missing images.
    RebackupMissingImages(PostQuery),
    /// Clean up invalid pictures (e.g., "image deleted" placeholders).
    CleanupInvalidPictures,
}

impl TaskRequest {
    pub fn total(&self) -> u32 {
        match self {
            TaskRequest::BackupFavorites(options) => options.num_pages,
            TaskRequest::BackupUser(options) => options.num_pages,
            TaskRequest::UnfavoritePosts => 1,
            TaskRequest::Export(_) => 1,
            TaskRequest::CleanupPictures(_) => 0,
            TaskRequest::CleanupAvatars => 0,
            TaskRequest::CleanupInvalidPosts(_) => 0,
            TaskRequest::RebackupPosts(_) => 0,
            TaskRequest::RebackupMissingImages(_) => 0,
            TaskRequest::CleanupInvalidPictures => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupInvalidPostsOptions {
    /// Whether to clean up posts that are valid themselves but their retweeted content is invalid.
    pub clean_retweeted_invalid: bool,
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
pub enum ResolutionPolicy {
    #[default]
    Highest,
    Lowest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPicturesOptions {
    pub policy: ResolutionPolicy,
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
pub enum SearchTerm {
    Fuzzy(String),
    Strict(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostQuery {
    pub user_id: Option<i64>,
    pub start_date: Option<i64>, // Unix timestamp
    pub end_date: Option<i64>,   // Unix timestamp
    pub search_term: Option<SearchTerm>,
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
#[serde(tag = "type", content = "data")]
pub enum AttachedImage {
    #[serde(rename = "livephoto")]
    LivePhoto { id: String, video_url: String },
    #[serde(rename = "video_cover")]
    VideoCover { id: String, video_url: String },
    #[serde(rename = "normal")]
    Normal { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInfo {
    pub post: Post,
    pub avatar_id: Option<String>,
    pub emoji_map: HashMap<String, String>,
    pub standalone_pics: Vec<AttachedImage>,
    pub inline_map: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedPostInfo {
    pub posts: Vec<PostInfo>,
    pub total_items: u64,
}
