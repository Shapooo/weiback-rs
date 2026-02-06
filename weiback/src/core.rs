pub mod post_processer;
pub mod task;
pub mod task_handler;
pub mod task_manager;

use bytes::Bytes;
use log::{debug, error, info, warn};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::{spawn, sync::mpsc};
use weibosdk_rs::{ApiClient as SdkApiClient, api_client::LoginState, session::Session};

#[cfg(not(feature = "dev-mode"))]
use crate::api::DefaultApiClient;
#[cfg(feature = "dev-mode")]
use crate::api::DevApiClient;
use crate::config::get_config;
use crate::error::Result;
use crate::exporter::ExporterImpl;
use crate::media_downloader::MediaDownloaderHandle;
use crate::message::{ErrType, Message};
use crate::models::User;
use crate::storage::StorageImpl;
pub use task::{
    BackupFavoritesOptions, BackupUserPostsOptions, ExportJobOptions, PaginatedPostInfo, PostQuery,
    Task, TaskContext, TaskRequest, UserPostFilter,
};
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
    msg_sender: mpsc::Sender<Message>,
}

impl Core {
    pub(crate) fn new(
        task_handler: TH,
        sdk_api_client: Arc<CurrentSdkApiClient>,
        msg_sender: mpsc::Sender<Message>,
    ) -> Result<Self> {
        Ok(Self {
            next_task_id: AtomicU64::new(1),
            task_handler: Arc::new(task_handler),
            task_manager: Arc::new(TaskManger::new()),
            sdk_api_client,
            msg_sender,
        })
    }

    pub fn task_manager(&self) -> Arc<TaskManger> {
        self.task_manager.clone()
    }

    pub fn get_my_uid(&self) -> Result<String> {
        Ok(self.sdk_api_client.session()?.uid.clone())
    }

    pub async fn export_posts(&self, options: ExportJobOptions) -> Result<()> {
        let ctx = self.create_task_context().await?;
        self.task_handler.export_posts(ctx, options).await
    }

    pub async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPostInfo> {
        let ctx = self.create_task_context().await?;
        self.task_handler.query_posts(ctx, query).await
    }

    pub async fn delete_post(&self, id: i64) -> Result<()> {
        let ctx = self.create_task_context().await?;
        self.task_handler.delete_post(ctx, id).await
    }

    pub async fn rebackup_post(&self, id: i64) -> Result<()> {
        let ctx = self.create_task_context().await?;
        self.task_handler.rebackup_post(ctx, id).await
    }

    pub async fn get_picture_blob(&self, id: String) -> Result<Option<Bytes>> {
        let ctx = self.create_task_context().await?;
        self.task_handler.get_picture_blob(ctx, &id).await
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
                let ctx = self.create_task_context().await?;
                spawn(async move {
                    if let Err(e) = th.save_user_info(ctx, &user).await {
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
        let ctx = self.create_task_context().await?;
        self.record_task(&ctx, request.clone()).await?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    pub async fn backup_favorites(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_task_context().await?;
        self.record_task(&ctx, request.clone()).await?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    pub async fn unfavorite_posts(&self) -> Result<()> {
        let ctx = self.create_task_context().await?;
        self.record_task(&ctx, TaskRequest::UnfavoritePosts).await?;
        spawn(handle_task_request(
            self.task_handler.clone(),
            ctx,
            TaskRequest::UnfavoritePosts,
        ));
        Ok(())
    }

    async fn create_task_context(&self) -> Result<Arc<TaskContext>> {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let ctx = Arc::new(TaskContext {
            task_id: id,
            config: get_config().read()?.clone(),
            msg_sender: self.msg_sender.clone(),
        });
        Ok(ctx)
    }

    async fn record_task(&self, ctx: &TaskContext, request: TaskRequest) -> Result<()> {
        info!(
            "Recording new task with id: {}, request: {:?}",
            ctx.task_id, request
        );
        let total = request.total() as u64;
        let task = Task {
            id: ctx.task_id,
            total,
            progress: 0,
            request,
        };
        self.task_manager.new_task(task.id, task)?;
        debug!("Task {} recorded successfully", ctx.task_id);
        Ok(())
    }
}

async fn handle_task_request(task_handler: Arc<TH>, ctx: Arc<TaskContext>, request: TaskRequest) {
    info!("Handling task request for task_id: {}", ctx.task_id);
    debug!("Task request details: {:?}", request);
    let res = match request {
        TaskRequest::BackupUser(options) => task_handler.backup_user(ctx.clone(), options).await,
        TaskRequest::UnfavoritePosts => task_handler.unfavorite_posts(ctx.clone()).await,
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(ctx.clone(), options).await
        }
    };
    if let Err(err) = res {
        error!("Task {} failed: {}", ctx.task_id, err);
        ctx.send_error(
            ErrType::LongTaskFail {
                task_id: ctx.task_id,
            },
            ctx.task_id,
            err.to_string(),
        )
        .await
        .unwrap_or_else(|e| {
            error!(
                "Failed to send error message for task {}: {}",
                ctx.task_id, e
            )
        });
    } else {
        info!("Task {} completed successfully", ctx.task_id);
    }
}
