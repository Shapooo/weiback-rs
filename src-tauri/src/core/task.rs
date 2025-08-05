use std::ops::RangeInclusive;

pub struct Task {
    pub id: u64,
    pub total: u64,
    pub progress: u64,
    pub request: TaskRequest,
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
