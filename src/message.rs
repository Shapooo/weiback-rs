use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub enum Task {
    DownloadPosts(RangeInclusive<u32>, bool, u8),
    ExportFromLocal(RangeInclusive<u32>, bool, u8),
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    InProgress(f32, String),
    Finished,
    Error(String),
    Info(String),
}

impl TaskStatus {
    pub fn is_inprogress(&self) -> bool {
        if let TaskStatus::InProgress(_, _) = self {
            true
        } else {
            false
        }
    }
    pub fn is_finished(&self) -> bool {
        if let TaskStatus::Finished = self {
            true
        } else {
            false
        }
    }
    pub fn is_error(&self) -> bool {
        if let TaskStatus::Error(_) = self {
            true
        } else {
            false
        }
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::InProgress(0.0, "".into())
    }
}
