use std::ops::RangeInclusive;
use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::{config::Config, message::Message};

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
