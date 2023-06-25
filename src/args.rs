use std::path::PathBuf;

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    /// location of config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}
