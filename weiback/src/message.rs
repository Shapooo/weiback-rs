use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct UserMeta {
    pub uid: String,
    pub name: String,
    pub posts_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    BackUser,
    BackFav,
    Unfav,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskProgress {
    pub r#type: TaskType,
    pub task_id: u64,
    pub total_increment: u64,
    pub progress_increment: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ErrType {
    DownMediaFail { url: String },
    LongTextFail { post_id: i64 },
    LongTaskFail { task_id: u64 },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrMsg {
    pub r#type: ErrType,
    pub task_id: u64,
    pub err: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Message {
    TaskProgress(TaskProgress),
    Err(ErrMsg),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UiAction {
    BackupUser {
        user_id: String,
        range: RangeInclusive<u32>,
    },
    BackupFavorites {
        range: RangeInclusive<u32>,
    },
    Export,
}
