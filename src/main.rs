pub mod args;
pub mod config;
pub mod fetched_data;
pub mod fetcher;
pub mod persister;
pub mod sql_data;
pub mod task_handler;

use anyhow::Result;
use config::get_config;
use log::{debug, info};
use simple_logger::SimpleLogger;
use task_handler::TaskHandler;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();
    info!("start running...");
    // load config
    let conf = get_config()?;
    debug!("config is {:?}", conf);

    let task_handler = TaskHandler::build(conf)?;
    task_handler.fetch_all_page().await?;

    info!("done");
    Ok(())
}
