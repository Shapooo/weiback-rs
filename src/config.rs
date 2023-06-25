use std::{env::current_exe, fs, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::Parser;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub web_cookie: String,
    #[serde(default)]
    pub mobile_cookie: String,
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub db: PathBuf,
}

const DEFAULT_DB_PATH_STR: &str = "res/weiback.db";

impl Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        if path.is_file() {
            let file = fs::read_to_string(path)?;
            Ok(serde_yaml::from_str::<Config>(&file)?)
        } else {
            return Err(anyhow!("config file is invalid"));
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            web_cookie: Default::default(),
            mobile_cookie: Default::default(),
            uid: Default::default(),
            db: Default::default(),
        }
    }
}

pub fn get_config() -> Result<Config> {
    let args = Args::parse();
    let config_file: Option<PathBuf> = args.config.or(current_exe().ok().map(|mut path| {
        path.pop();
        path.push("config.yaml");
        path
    }));

    if config_file.is_none() {
        panic!("config file must be set!");
    }

    let config_file = config_file.unwrap();
    info!("loading config from: {:?}", config_file);

    let mut conf = Config::load(&config_file)?;

    if conf.db.as_os_str() == "" {
        let mut exe = current_exe().unwrap();
        exe.pop();
        exe.push(DEFAULT_DB_PATH_STR);
        conf.db = exe;
    }

    debug!("config loaded: {:?}", conf);
    Ok(conf)
}
