use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    /// location of config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// location of database file
    #[arg(short, long)]
    pub db: Option<String>,
}
