// use crate::utils::parse_file;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    /// cookie of web page
    #[arg(short, long)]
    pub web_cookie: Option<String>,
    /// cookie of mobile web page
    #[arg(short, long)]
    pub mobile_cookie: Option<String>,
    /// user id
    #[arg(short, long)]
    pub uid: Option<String>,
    /// location of config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// location of database file
    #[arg(short, long)]
    pub db: Option<PathBuf>,
}
