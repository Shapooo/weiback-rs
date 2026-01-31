use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    config::Config,
    message::{ErrMsg, ErrType, Message, TaskProgress, TaskType},
    models::Post,
};

pub struct Task {
    pub id: u64,
    pub total: u64,
    pub progress: u64,
    pub request: TaskRequest,
}

#[derive(Debug)]
pub struct TaskContext {
    pub task_id: u64,
    pub config: Config,
    pub msg_sender: mpsc::Sender<Message>,
}

impl TaskContext {
    pub async fn send_progress(
        &self,
        r#type: TaskType,
        total_increment: u64,
        progress_increment: u64,
    ) -> Result<(), mpsc::error::SendError<Message>> {
        self.msg_sender
            .send(Message::TaskProgress(TaskProgress {
                r#type,
                task_id: self.task_id,
                total_increment,
                progress_increment,
            }))
            .await
    }

    pub async fn send_error(
        &self,
        r#type: ErrType,
        task_id: u64,
        err: String,
    ) -> Result<(), mpsc::error::SendError<Message>> {
        self.msg_sender
            .send(Message::Err(ErrMsg {
                r#type,
                task_id,
                err,
            }))
            .await
    }
}

#[derive(Debug, Clone)]
pub enum TaskRequest {
    // to download favorites (range, with pic, image definition level)
    BackupFavorites(BackupFavoritesOptions),
    // to unfavorite favorite post
    UnfavoritePosts,
    // to backup user (id, with pic, image definition level)
    BackupUser(BackupUserPostsOptions),
}

impl TaskRequest {
    pub fn total(&self) -> u32 {
        match self {
            TaskRequest::BackupFavorites(options) => options.num_pages,
            TaskRequest::BackupUser(options) => options.num_pages,
            TaskRequest::UnfavoritePosts => 1,
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
