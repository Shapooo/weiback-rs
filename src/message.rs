use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub enum Task {
    DownloadMeta(RangeInclusive<u32>),
    DownloadWithPic(RangeInclusive<u32>),
    ExportFromNet(RangeInclusive<u32>),
    ExportFromLocal(RangeInclusive<u32>, bool),
}

#[derive(Debug, Clone)]
pub enum Progress {
    InProgress(f32, String),
    Finished,
    Error(String),
}

impl Progress {
    pub fn is_inprogress(&self) -> bool {
        if let Progress::InProgress(_, _) = self {
            true
        } else {
            false
        }
    }
    pub fn is_finished(&self) -> bool {
        if let Progress::Finished = self {
            true
        } else {
            false
        }
    }
    pub fn is_error(&self) -> bool {
        if let Progress::Error(_) = self {
            true
        } else {
            false
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::InProgress(0.0, "".into())
    }
}
