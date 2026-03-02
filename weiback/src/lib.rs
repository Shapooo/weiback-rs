//! `weiback` is a Rust library designed for archiving Weibo data.
//!
//! It provides functionalities to interact with the Weibo API,
//! store user posts and media, generate HTML exports, and manage
//! the local data. The library is structured into modules covering
//! API interactions, data storage, core logic, data models, and utility functions.

pub mod api;
pub mod builder;
pub mod config;
pub mod core;
pub mod emoji_map;
pub mod error;
pub mod exporter;
pub mod html_generator;
pub mod image_validator;
pub mod media_downloader;
pub mod message;
pub mod models;
pub mod storage;
pub mod utils;

#[cfg(test)]
pub mod mock;

#[cfg(feature = "dev-mode")]
pub mod dev_client;

#[cfg(feature = "internal-models")]
pub mod internals {
    pub use crate::api::internal::{page_info, url_struct};
    pub use crate::storage::{database, internal as storage_internal, picture_storage};
}
