use std::ops::RangeInclusive;

use tauri;

use crate::error::Result;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn unfavorite_posts() {
    todo!()
}

#[tauri::command]
async fn backup_user(uid: i32, range:RangeInclusive<u32>) {
    todo!()
}

#[tauri::command]
async fn backup_favorites(range: RangeInclusive<u32>) {
    todo!()
}

#[tauri::command]
async fn export_from_local(range: RangeInclusive<u32>) {}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            unfavorite_posts,
            backup_user,
            backup_favorites,
            export_from_local
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
