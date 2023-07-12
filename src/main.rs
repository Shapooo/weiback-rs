use anyhow::Result;
use log::info;
use simple_logger::SimpleLogger;

use weiback_rs::core::Core;

fn main() -> Result<()> {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Warn)
        .with_module_level("sqlx", log::LevelFilter::Error)
        .env()
        .init()
        .unwrap();
    info!("start running...");
    let core = Core::new();
    core.run();

    info!("done");
    Ok(())
}
