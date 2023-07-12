use anyhow::Result;
use log::{debug, info};
use simple_logger::SimpleLogger;

use weiback_rs::config::get_config;
use weiback_rs::core::WbApp;

fn main() -> Result<()> {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Warn)
        .with_module_level("sqlx", log::LevelFilter::Error)
        .env()
        .init()
        .unwrap();
    info!("start running...");
    // load config
    let conf = get_config()?;
    debug!("config is {:?}", conf);

    let gui = WbApp::new(conf);
    gui.run();

    info!("done");
    Ok(())
}
