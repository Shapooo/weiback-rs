//! This module provides the [`CoreBuilder`] for orchestrating the initialization of the application.
//!
//! The builder handles the complex setup process of all internal components, including:
//! - Database connection pooling and storage initialization.
//! - HTTP client and media downloader worker lifecycle management.
//! - Weibo API client configuration (supporting both standard and development modes).
//! - Task handler and core service assembly.

use std::sync::Arc;

use tracing::info;
use weibosdk_rs::{ApiClient as SdkApiClient, Client as HttpClient};

use crate::{
    config::get_config,
    core::{Core, task_handler::TaskHandler},
    error::Result,
    exporter::ExporterImpl,
    media_downloader::create_downloader,
    storage::{StorageImpl, database},
};
use tokio::runtime::Runtime;

#[cfg(not(feature = "dev-mode"))]
use crate::api::DefaultApiClient;
#[cfg(feature = "dev-mode")]
use crate::{api::DevApiClient, dev_client::DevClient};

const DOWNLOADER_BUFFER_SIZE: usize = 100;

/// A builder for creating and configuring the [`Core`] service.
///
/// `CoreBuilder` follows the builder pattern to encapsulate the logic required to
/// properly initialize all dependencies and sub-systems before the application starts.
pub struct CoreBuilder;

impl CoreBuilder {
    /// Creates a new instance of `CoreBuilder`.
    pub fn new() -> Self {
        Self
    }

    /// Builds and returns an initialized [`Core`] instance wrapped in an [`Arc`].
    ///
    /// This method performs the following steps:
    /// 1. Reads the global configuration.
    /// 2. Initializes the database pool and [`StorageImpl`].
    /// 3. Sets up the [`ExporterImpl`].
    /// 4. Configures the [`HttpClient`] and spawns the [`MediaDownloader`] worker thread/task.
    /// 5. Initializes the appropriate API client (Standard or DevMode).
    /// 6. Assembles the [`TaskHandler`] and finally the [`Core`] service.
    ///
    /// # Errors
    /// Returns a [`Result`] if any component fails to initialize (e.g., database connection error).
    pub fn build(self) -> Result<Arc<Core>> {
        info!("CoreBuilder: Building Core service...");
        let main_config = get_config();
        let main_config_read_guard = main_config.read()?;

        let db_pool = Runtime::new()?.block_on(database::create_db_pool())?;
        let storage = StorageImpl::new(db_pool);
        info!("Storage initialized");

        let exporter = ExporterImpl::new();
        info!("Exporter initialized");

        let http_client = HttpClient::new()?;
        info!("HTTP client created");

        let (handle, worker) =
            create_downloader(DOWNLOADER_BUFFER_SIZE, http_client.main_client().clone());
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(worker.run());
        } else {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(worker.run());
            });
        }
        info!("MediaDownloader initialized and worker spawned");

        #[cfg(feature = "dev-mode")]
        let (sdk_api_client, api_client) = {
            let dev_client =
                DevClient::new(http_client, main_config_read_guard.dev_mode_out_dir.clone());
            let sdk_api_client = SdkApiClient::new(dev_client, main_config_read_guard.sdk_config);
            let api_client = DevApiClient::new(sdk_api_client.clone());
            (Arc::new(sdk_api_client), api_client)
        };
        #[cfg(not(feature = "dev-mode"))]
        let (sdk_api_client, api_client) = {
            let sdk_api_client =
                SdkApiClient::new(http_client, main_config_read_guard.sdk_config.clone());
            let api_client = DefaultApiClient::new(sdk_api_client.clone());
            (Arc::new(sdk_api_client), api_client)
        };
        info!("ApiClient and SdkApiClient initialized");

        let task_handler = TaskHandler::new(api_client, storage, exporter, handle)?;
        info!("TaskHandler initialized");

        let core = Arc::new(Core::new(task_handler, sdk_api_client)?);
        info!("Core service built successfully.");

        Ok(core)
    }
}

impl Default for CoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}
