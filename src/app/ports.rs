use std::ops::RangeInclusive;

use anyhow::Error;
use egui::ImageData;

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
pub enum TaskResponse {
    SumOfFavDB(u32, u32),             // remain sum of favorite in weibo and local db
    UserMeta(i64, String, ImageData), // screen name and avatar picture
    InProgress(f32, String),          // long time task is in progress
    Finished(u32, u32),               // long time task is finished
    Error(Error),                     // error occurs
}
