use crate::args::Args;
use anyhow::{anyhow, Result};
use clap::Parser;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::{env::current_exe, fs, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub web_cookie: String,
    #[serde(default)]
    pub mobile_cookie: String,
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub db: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            web_cookie: Default::default(),
            mobile_cookie: Default::default(),
            uid: Default::default(),
            db: Default::default(),
        }
    }

    pub fn try_load(&mut self, path: &PathBuf) -> Result<()> {
        if path.is_file() {
            let file = fs::read_to_string(path)?;
            *self = serde_yaml::from_str::<Config>(&file)?;
            Ok(())
        } else {
            return Err(anyhow!("config file is invalid"));
        }
    }
}

pub fn get_config() -> Result<Config> {
    let mut conf = Config::new();
    let args = Args::parse();
    let config_file: Option<PathBuf> = args.config.or(current_exe().ok().map(|mut exe| {
        exe.pop();
        exe.push("config.yaml");
        exe
    }));

    if config_file.is_none() {
        panic!("config file must be set!");
    }

    let config_file = config_file.unwrap();
    info!("loading config from: {:?}", config_file);

    match conf.try_load(&config_file) {
        Ok(_) => debug!("config loaded: {:?}", conf),
        Err(err) => panic!("cannot load config file: {err}"),
    }

    conf.db = args.db.unwrap_or(conf.db);
    if conf.db.is_empty() {
        panic!("database file must be set!");
    }

    info!("conf loaded");
    Ok(conf)
}
