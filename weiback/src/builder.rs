use std::sync::Arc;

use log::info;
use tokio::sync::mpsc;
use weibosdk_rs::{ApiClient as SdkApiClient, Client as HttpClient};

use crate::{
    config::get_config,
    core::{Core, task_handler::TaskHandler},
    error::Result,
    exporter::ExporterImpl,
    media_downloader::create_downloader,
    message::Message,
    storage::{StorageImpl, database},
};
use tokio::runtime::Runtime;

#[cfg(not(feature = "dev-mode"))]
use crate::api::DefaultApiClient;
#[cfg(feature = "dev-mode")]
use crate::{api::DevApiClient, dev_client::DevClient};

const DOWNLOADER_BUFFER_SIZE: usize = 100;
const MESSAGE_BUFFER_SIZE: usize = 100;

pub struct CoreBuilder;

impl CoreBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build(self) -> Result<(Arc<Core>, mpsc::Receiver<Message>)> {
        info!("CoreBuilder: Building Core service...");
        let main_config = get_config();
        let main_config_read_guard = main_config.read()?;

        let (msg_sender, msg_receiver) = mpsc::channel(MESSAGE_BUFFER_SIZE);
        info!("MPSC channel created");

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
            let sdk_api_client =
                SdkApiClient::new(dev_client, main_config_read_guard.sdk_config.clone());
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

        let core = Arc::new(Core::new(task_handler, sdk_api_client, msg_sender)?);
        info!("Core service built successfully.");

        Ok((core, msg_receiver))
    }
}

impl Default for CoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}
