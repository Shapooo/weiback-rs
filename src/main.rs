use anyhow::Result;
use log::{debug, info};
use simple_logger::SimpleLogger;

use weiback_rs::config::get_config;
use weiback_rs::task_handler::TaskHandler;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new()
        .env()
        .with_colors(true)
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();
    info!("start running...");
    // load config
    let conf = get_config()?;
    debug!("config is {:?}", conf);

    let task_handler = TaskHandler::build(conf).await?;
    task_handler.fetch_all_page().await?;

    info!("done");
    Ok(())
}
