#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use anyhow::Result;
use env_logger::Builder;
use log::{info, LevelFilter};

use weiback_rs::core::Core;

fn main() -> Result<()> {
    let log_path = std::env::current_exe()?;
    let log_path = log_path
        .parent()
        .ok_or(anyhow::anyhow!(
            "the executable: {:?} should have parent, maybe bugs in there",
            std::env::current_exe()
        ))?
        .join("res/weiback.log");
    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(log_path)?;
    Builder::new()
        .filter_level(LevelFilter::Warn)
        .parse_default_env()
        .filter_module("sqlx", LevelFilter::Error)
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    info!("start running...");
    let core = Core::new();
    core.run()?;

    info!("done");
    Ok(())
}
