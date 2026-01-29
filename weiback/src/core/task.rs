use std::ops::RangeInclusive;
use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::{
    config::Config,
    message::{ErrMsg, ErrType, Message, TaskProgress, TaskType},
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
    BackupFavorites(BFOptions),
    // to unfavorite favorite post
    UnfavoritePosts,
    // to backup user (id, with pic, image definition level)
    BackupUser(BUOptions),
}

impl TaskRequest {
    pub fn total(&self) -> u32 {
        match self {
            TaskRequest::BackupFavorites(o) => {
                let range = o.range.to_owned();
                range.end() - range.start() + 1
            }
            TaskRequest::BackupUser(o) => {
                let range = o.range.to_owned();
                range.end() - range.start() + 1
            }
            TaskRequest::UnfavoritePosts => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BFOptions {
    pub range: RangeInclusive<u32>,
}

#[derive(Debug, Clone)]
pub struct BUOptions {
    pub range: RangeInclusive<u32>,
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

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub export_dir: PathBuf,
    pub task_name: String,
    pub reverse: bool,
    pub range: RangeInclusive<u32>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            export_dir: PathBuf::from("."),
            task_name: "weiback_export".to_string(),
            reverse: false,
            range: 0..=1_000_000_000,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PaginatedPosts {
    pub posts: Vec<crate::models::Post>,
    pub total_items: u64,
}
