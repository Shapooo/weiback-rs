use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use log::debug;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use crate::error::{Error, Result};
use crate::models::PictureDefinition;

// 使用 OnceCell 替代 Lazy，以支持可能失败的、显式的初始化。
static CONFIG: OnceCell<Arc<RwLock<Config>>> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub db_path: PathBuf,
    pub templates_path: PathBuf,
    pub session_path: PathBuf,
    pub download_pictures: bool,
    pub picture_definition: PictureDefinition,
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(PathBuf::new)
            .join("weiback");
        let data_dir = dirs::data_dir()
            .unwrap_or_else(PathBuf::new)
            .join("weiback");
        Self {
            db_path: config_dir.join("weiback.db"),
            templates_path: data_dir.join("templates"),
            session_path: config_dir.join("session.json"),
            download_pictures: true,
            picture_definition: Default::default(),
        }
    }
}

/// 显式初始化函数，应在 main 函数开始时调用。
///
/// 它会尝试加载配置文件。如果所有路径都找不到配置文件，
/// 它将创建一个默认配置并尝试将其写入用户本地配置目录。
///
/// # Errors
///
/// 如果配置文件存在但无法读取，或者在尝试写入新的默认配置文件时发生 I/O 错误，
/// 此函数将返回一个错误。
pub fn init() -> Result<()> {
    let config = load_or_create()?;
    // set 如果已经初始化，会返回 Err，这里我们忽略这个错误，因为这意味着已经有别的线程初始化了。
    let _ = CONFIG.set(Arc::new(RwLock::new(config)));
    Ok(())
}

/// 获取全局配置实例。
///
/// 这是一个健壮的函数，保证总能返回一个配置实例。
///
/// - 如果 `init()` 已经被成功调用，它将返回 `init()` 设置的配置。
/// - 如果 `init()` 从未被调用，它将首次尝试从文件加载配置（但不会创建新文件）。
///   如果加载失败（任何原因），它将回退到内存中的默认配置，并确保程序不会崩溃。
pub fn get_config() -> Arc<RwLock<Config>> {
    CONFIG
        .get_or_init(|| {
            // "隐式"初始化路径：尝试加载，如果失败（包括未找到、权限问题等），
            // 则使用默认值。这保证了 get_config 总能成功返回，不会 panic。
            // 这里不写入文件，以避免在运行时产生不可控的 I/O 错误。
            let config = load_from_files().unwrap_or_default();
            Arc::new(RwLock::new(config))
        })
        .clone()
}

// 尝试从所有已知路径加载配置。
fn load_from_files() -> Result<Config> {
    let config_path =
        find_config_file()?.ok_or(Error::Other("config file not found".to_string()))?;
    let content = fs::read_to_string(config_path)?;
    Ok(from_str(&content)?)
}

// 尝试加载配置，如果找不到，则创建并保存一个新的默认配置。
fn load_or_create() -> Result<Config> {
    if let Some(path) = find_config_file()? {
        let content = fs::read_to_string(path)?;
        return Ok(from_str(&content)?);
    }

    // 未找到配置文件，创建并写入默认配置
    let config = Config::default();
    let config_local_path = dirs::config_local_dir()
        .unwrap_or_else(PathBuf::new)
        .join("weiback/config.json");

    if let Some(parent) = config_local_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config_local_path, serde_json::to_string_pretty(&config)?)?;
    debug!(
        "Default configuration file created at: {:?}",
        config_local_path
    );

    Ok(config)
}

// 在预设的路径中查找存在的配置文件。
fn find_config_file() -> Result<Option<PathBuf>> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap_or(&exe_path);

    let paths = [
        dirs::config_local_dir()
            .unwrap_or_default()
            .join("weiback/config.json"),
        dirs::config_dir()
            .unwrap_or_default()
            .join("weiback/config.json"),
        exe_dir.join("weiback/config.json"),
    ];

    Ok(paths.into_iter().find(|p| p.exists()))
}
