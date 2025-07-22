use egui::ImageData;

use crate::error::Error;

pub enum Message {
    SumOfFavDB(u32, u32),             // remain sum of favorite in weibo and local db
    UserMeta(i64, String, ImageData), // screen name and avatar picture
    InProgress(f32, String),          // long time task is in progress
    Finished(u32, u32),               // long time task is finished
    Error(Error),                     // error occurs
}
