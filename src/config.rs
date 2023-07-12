use std::{
    env::current_exe,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use clap::Parser;
use lazy_static::lazy_static;
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
lazy_static! {
    static ref DEFAULT_CONF_PATH: PathBuf =
        current_exe().unwrap().join(&Path::new("res/config.yaml"));
}

impl Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load(path: &PathBuf) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        } else if !path.is_file() {
            return Err(anyhow!("config file is invalid"));
        }
        let file = fs::read_to_string(path)?;
        Ok(Some(serde_yaml::from_str::<Config>(&file)?))
    }

    pub fn save(&self) -> Result<()> {
        let config_content = serde_yaml::to_string(self)?;
        Ok(fs::write(&*DEFAULT_CONF_PATH, config_content)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            web_cookie: Default::default(),
            mobile_cookie: Default::default(),
            uid: Default::default(),
            db: DEFAULT_DB_PATH_STR.into(),
        }
    }
}

pub fn get_config() -> Result<Option<Config>> {
    let args = Args::parse();
    let config_file: Option<PathBuf> = args.config.or(current_exe().ok().map(|mut path| {
        path.pop();
        path.push("config.yaml");
        path
    }));

    let Some(config_file) = config_file else {
        return Ok(None);
    };
    info!("loading config from: {:?}", config_file);

    let Some(conf) = Config::load(&config_file) ? else {
        return Ok(None);
    };

    if conf.db.as_os_str() == ""
        || conf.uid == ""
        || conf.mobile_cookie == ""
        || conf.web_cookie == ""
    {
        info!("some item field are missing in config file");
        return Ok(None);
    }

    debug!("config loaded: {:?}", conf);
    Ok(Some(conf))
}
