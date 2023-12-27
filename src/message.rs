use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub enum Task {
    DownloadFav(RangeInclusive<u32>, bool, u8),
    ExportFromLocal(RangeInclusive<u32>, bool, u8),
    UnfavoritePosts(RangeInclusive<u32>),
    DownloadPosts(i64, RangeInclusive<u32>, bool, u8),
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Init(u32, u32),
    InProgress(f32, String),
    Finished(u32, u32),
    Error(String),
}

#[allow(unused)]
impl TaskStatus {
    pub fn is_inprogress(&self) -> bool {
        matches!(self, Self::InProgress(_, _))
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Finished(_, _))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::InProgress(0.0, "".into())
    }
}
