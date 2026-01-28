pub mod post_processer;
pub mod task;
pub mod task_handler;
pub mod task_manager;

use log::{debug, error, info, warn};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::spawn;
use weibosdk_rs::{ApiClient as SdkApiClient, api_client::LoginState, session::Session};

#[cfg(not(feature = "dev-mode"))]
use crate::api::DefaultApiClient;
#[cfg(feature = "dev-mode")]
use crate::api::DevApiClient;
use crate::config::get_config;
use crate::error::Result;
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderHandle;
use crate::message::{ErrMsg, ErrType, Message};
use crate::models::User;
use crate::storage::StorageImpl;
pub use task::{BFOptions, BUOptions, ExportOptions, Task, TaskRequest, UserPostFilter};
pub use task_handler::TaskHandler;
use task_manager::TaskManger;

#[cfg(not(feature = "dev-mode"))]
type TH = TaskHandler<DefaultApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;
#[cfg(feature = "dev-mode")]
type TH = TaskHandler<DevApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;

#[cfg(feature = "dev-mode")]
type CurrentSdkApiClient = SdkApiClient<crate::dev_client::DevClient>;
#[cfg(not(feature = "dev-mode"))]
type CurrentSdkApiClient = SdkApiClient<weibosdk_rs::Client>;

pub struct Core {
    next_task_id: AtomicU64,
    task_handler: Arc<TH>,
    task_manager: Arc<TaskManger>,
    sdk_api_client: Arc<CurrentSdkApiClient>,
}

impl Core {
    pub fn new(task_handler: TH, sdk_api_client: Arc<CurrentSdkApiClient>) -> Result<Self> {
        Ok(Self {
            next_task_id: AtomicU64::new(1),
            task_handler: Arc::new(task_handler),
            task_manager: Arc::new(TaskManger::new()),
            sdk_api_client,
        })
    }

    pub fn task_manager(&self) -> Arc<TaskManger> {
        self.task_manager.clone()
    }

    pub fn get_my_uid(&self) -> Result<String> {
        Ok(self.sdk_api_client.session()?.uid.clone())
    }

    pub async fn export_from_local(&self, options: ExportOptions) -> Result<()> {
        self.task_handler.export_from_local(options).await
    }

    pub async fn get_username_by_id(&self, uid: i64) -> Result<Option<String>> {
        self.task_handler
            .get_user(uid)
            .await
            .map(|opt| opt.map(|u| u.screen_name))
    }

    pub async fn get_sms_code(&self, phone_number: String) -> Result<()> {
        info!(
            "send_code called for phone number (partially hidden): ...{}",
            &phone_number.chars().skip(7).collect::<String>()
        );
        match self.sdk_api_client.login_state() {
            LoginState::LoggedIn { .. } => {
                warn!("Already logged in, skipping send_code.");
                Ok(())
            }
            _ => {
                info!("Sending SMS code.");
                self.sdk_api_client.get_sms_code(phone_number).await?;
                Ok(())
            }
        }
    }

    pub async fn login(&self, sms_code: String) -> Result<()> {
        info!("login called with a sms_code");
        match self.sdk_api_client.login_state() {
            LoginState::WaitingForCode { .. } => {
                info!("Attempting to login with SMS code.");
                self.sdk_api_client.login(&sms_code).await?;
                info!("Login successful.");
                let session_path = get_config()
                    .read()
                    .expect("config lock failed")
                    .session_path
                    .clone();
                let session = self.sdk_api_client.session()?;
                session.save(session_path)?;

                let user: User = serde_json::from_value(session.user)?;
                let user_id = user.id;

                let th = self.task_handler.clone();
                spawn(async move {
                    if let Err(e) = th.save_user_info(&user).await {
                        error!("Save user info failed: {e}");
                    }
                });
                info!("Logged in user {} saved.", user_id);

                Ok(())
            }
            LoginState::LoggedIn { .. } => {
                warn!("Already logged in, skipping login.");
                Ok(())
            }
            LoginState::Init => {
                error!("Wrong login state to login: Init");
                Err(crate::error::Error::InconsistentTask(
                    "FATAL: wrong login state to login".to_string(),
                ))
            }
        }
    }

    pub async fn login_state(&self) -> Result<Option<Value>> {
        info!("get login state");
        Ok(self.sdk_api_client.session().ok().map(|s| s.user))
    }

    pub async fn login_with_session(&self) -> Result<()> {
        let session_path = get_config().read()?.session_path.clone();
        if let Ok(session) = Session::load(session_path.as_path()) {
            let api_client = self.sdk_api_client.clone();
            if let Err(e) = api_client.login_with_session(session).await {
                error!("login with session failed: {e}");
            }
            info!("login with session successfully");
            match api_client.session() {
                Ok(session) => {
                    if let Err(e) = session.save(session_path) {
                        error!("save new session failed: {e}");
                    }
                }
                Err(e) => {
                    error!("get new session failed: {e}");
                }
            }
        }
        Ok(())
    }

    pub async fn backup_user(&self, request: TaskRequest) -> Result<()> {
        let id = self.record_task(request.clone()).await?;
        spawn(handle_task_request(self.task_handler.clone(), id, request));
        Ok(())
    }

    pub async fn backup_favorites(&self, request: TaskRequest) -> Result<()> {
        let id = self.record_task(request.clone()).await?;
        spawn(handle_task_request(self.task_handler.clone(), id, request));
        Ok(())
    }

    pub async fn unfavorite_posts(&self) -> Result<()> {
        let id = self.record_task(TaskRequest::UnfavoritePosts).await?;
        spawn(handle_task_request(
            self.task_handler.clone(),
            id,
            TaskRequest::UnfavoritePosts,
        ));
        Ok(())
    }

    async fn record_task(&self, request: TaskRequest) -> Result<u64> {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        info!("Recording new task with id: {id}, request: {request:?}");
        let total = request.total() as u64;
        let task = Task {
            id,
            total,
            progress: 0,
            request,
        };
        self.task_manager.new_task(id, task)?;
        debug!("Task {id} recorded successfully");
        Ok(id)
    }
}

async fn handle_task_request(task_handler: Arc<TH>, task_id: u64, request: TaskRequest) {
    info!("Handling task request for task_id: {task_id}");
    debug!("Task request details: {request:?}");
    let res = match request {
        TaskRequest::BackupUser(options) => task_handler.backup_user(task_id, options).await,
        TaskRequest::UnfavoritePosts => task_handler.unfavorite_posts(task_id).await,
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(task_id, options).await
        }
    };
    if let Err(err) = res {
        error!("Task {task_id} failed: {err}");
        task_handler
            .msg_sender()
            .send(Message::Err(ErrMsg {
                r#type: ErrType::LongTaskFail { task_id },
                task_id,
                err: err.to_string(),
            }))
            .await
            .unwrap_or_else(|e| error!("Failed to send error message for task {task_id}: {e}"));
    } else {
        info!("Task {task_id} completed successfully");
    }
}
