//! This module defines data structures for communication between the backend and UI.
//!
//! These structures are typically serialized and sent as events (e.g., through Tauri's
//! event system) to notify the frontend of task progress, errors, or other state changes.
//! They can also represent actions sent from the UI to the backend.
//!
//! **Note**: Some of these structures may overlap with types defined in other modules
//! like `core::task_manager`. They are kept separate to decouple UI messaging from
//! internal core logic.

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

/// Basic metadata about a Weibo user.
#[derive(Debug, Clone)]
pub struct UserMeta {
    pub uid: String,
    pub name: String,
    pub posts_count: u32,
}

/// The category of a task, for use in UI messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// Backing up a user's posts.
    BackUser,
    /// Backing up favorited posts.
    BackFav,
    /// Unfavoriting posts.
    Unfav,
}

/// A message indicating a change in task progress.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskProgress {
    pub r#type: TaskType,
    pub task_id: u64,
    pub total_increment: u64,
    pub progress_increment: u64,
}

/// The category of a non-fatal error reported to the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ErrType {
    /// Failure to download a media file. Contains the URL.
    DownMediaFail { url: String },
    /// Failure to fetch the full content of a long post. Contains the post ID.
    LongTextFail { post_id: i64 },
    /// A fatal error occurred in a long-running task. Contains the task ID.
    LongTaskFail { task_id: u64 },
}

/// A message containing details about a non-fatal error.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrMsg {
    pub r#type: ErrType,
    pub task_id: u64,
    pub err: String,
}

/// A generic message sent from the backend to the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Message {
    /// A task progress update.
    TaskProgress(TaskProgress),
    /// A non-fatal error notification.
    Err(ErrMsg),
}

/// Represents an action initiated from the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UiAction {
    /// Request to back up a specific user's posts.
    BackupUser {
        user_id: String,
        range: RangeInclusive<u32>,
    },
    /// Request to back up the current user's favorites.
    BackupFavorites { range: RangeInclusive<u32> },
    /// Request to export local data.
    Export,
}
