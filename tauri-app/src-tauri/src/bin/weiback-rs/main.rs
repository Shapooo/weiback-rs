#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use std::fs;

use anyhow::Result;
use env_logger::Builder;
use log::{LevelFilter, info};

fn main() -> Result<()> {
    init_logger()?;

    info!("start running...");
    tauri_app::run()?;

    info!("done");
    Ok(())
}

fn init_logger() -> Result<()> {
    let log_path = dirs::data_dir()
        .unwrap_or_default()
        .join("weiback")
        .join("weiback.log");
    fs::create_dir_all(log_path.parent().unwrap())?;
    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(log_path)?;
    Builder::new()
        .filter_level(LevelFilter::Debug)
        .parse_default_env()
        .filter_module("sqlx", LevelFilter::Error)
        .filter_module("h2", LevelFilter::Warn)
        .filter_module("hyper_util", LevelFilter::Warn)
        .filter_module("reqwest", LevelFilter::Warn)
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    Ok(())
}
