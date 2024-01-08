use std::ops::RangeInclusive;

use anyhow::Error;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Task {
    // to fetch user meta data, include screen name and avatar
    FetchUserMeta(i64),
    // to download favorites (range, with pic, image definition level)
    DownloadFav(RangeInclusive<u32>, bool, u8),
    // to export favorites from local db (range, with pic, image definition level)
    ExportFromLocal(RangeInclusive<u32>, bool, u8),
    // to unfavorite favorite post
    UnfavoritePosts,
    // to backup user (id, with pic, image definition level)
    BackupUser(i64, bool, u8),
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
