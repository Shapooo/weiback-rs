use crate::args::Args;
use anyhow::{anyhow, Result};
use clap::Parser;
use dirs::config_local_dir;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::{env::current_dir, fs, path::PathBuf};

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
        if path.exists() {
            let file = fs::read_to_string(path)?;
            *self = serde_yaml::from_str::<Config>(&file)?;
            Ok(())
        } else {
            return Err(anyhow!("config file not found"));
        }
    }
}

pub fn get_config() -> Result<Config> {
    let mut conf = Config::new();
    let args = Args::parse();
    let config_file = match args.config {
        Some(file) => {
            let file = if file.is_relative() {
                current_dir()?.join(file)
            } else {
                file
            };
            debug!("set config file path to {}", file.display());
            Some(file)
        }
        None => {
            let cur_dir_config = std::env::current_dir()?.join("config.yaml");
            if cur_dir_config.is_file() {
                debug!("set config file path to {}", cur_dir_config.display());
                Some(cur_dir_config)
            } else {
                let home_config = if cfg!(target_os = "windows") {
                    config_local_dir().unwrap().join("weiback\\config.yaml")
                } else {
                    config_local_dir().unwrap().join("weiback/config.yaml")
                };
                if home_config.is_file() {
                    debug!("set config file path to {}", home_config.display());
                    Some(home_config)
                } else {
                    None
                }
            }
        }
    };

    match config_file {
        Some(config_file) => {
            info!("load config from: {}", config_file.display());
            conf.try_load(&config_file)?;
            debug!("config: {:?}", conf);
            if conf.db.is_relative() {
                conf.db = config_file.parent().unwrap().join(conf.db);
                debug!("parse db path from config: {}", conf.db.display());
            }
        }
        None => {
            warn!("cannot found config file");
        }
    }

    if let Some(p) = args.web_cookie {
        debug!("cli set web_cookie: {}", p);
        conf.web_cookie = p;
    }
    if let Some(p) = args.mobile_cookie {
        debug!("cli set mobile cookie: {}", p);
        conf.mobile_cookie = p;
    }
    if let Some(p) = args.uid {
        debug!("cli set uid: {}", p);
        conf.uid = p;
    }
    if let Some(p) = args.db {
        debug!("cli set db file path to {}", p.display());
        if p.is_relative() {
            conf.db = std::env::current_dir()?.join(p);
        } else {
            conf.db = p;
        }
        debug!("parse db path from command line: {}", conf.db.display());
    }

    if conf.web_cookie.is_empty() {
        error!("cannot find web cookie from config file or command line");
        Err(anyhow!("web cookie must be setted"))
    } else if conf.uid.is_empty() {
        error!("cannot parse uid from config file or command line");
        Err(anyhow!("uid must be setted"))
    } else {
        info!("conf loaded");
        Ok(conf)
    }
}
