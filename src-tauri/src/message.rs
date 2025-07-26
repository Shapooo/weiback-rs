use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct UserMeta {
    pub uid: String,
    pub name: String,
    pub posts_count: u32,
}

#[derive(Debug, Clone)]
pub struct TaskProgress {
    pub id: u64,
    pub total_increment: u64,
    pub current_increment: u64,
}

#[derive(Debug)]
pub enum Message {
    UserMeta(UserMeta),
    TaskProgress(TaskProgress),
    Err(Error),
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
