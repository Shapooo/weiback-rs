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
