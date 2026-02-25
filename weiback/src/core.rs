pub mod post_processer;
pub mod task;
pub mod task_handler;
pub mod task_manager;

use bytes::Bytes;
use log::{debug, error, info, warn};
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
use crate::models::User;
use crate::storage::StorageImpl;
pub use task::{
    BackupFavoritesOptions, BackupUserPostsOptions, ExportJobOptions, PaginatedPostInfo, PostQuery,
    TaskContext, TaskRequest, UserPostFilter,
};
pub use task_handler::TaskHandler;
use task_manager::{SubTaskError, Task, TaskManager, TaskType};

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
    task_manager: Arc<TaskManager>,
    sdk_api_client: Arc<CurrentSdkApiClient>,
}

impl Core {
    pub(crate) fn new(task_handler: TH, sdk_api_client: Arc<CurrentSdkApiClient>) -> Result<Self> {
        Ok(Self {
            next_task_id: AtomicU64::new(1),
            task_handler: Arc::new(task_handler),
            task_manager: Arc::new(TaskManager::new()),
            sdk_api_client,
        })
    }

    pub async fn get_current_task(&self) -> Result<Option<Task>> {
        self.task_manager.get_current()
    }

    pub fn get_and_clear_sub_task_errors(&self) -> Result<Vec<SubTaskError>> {
        self.task_manager.get_and_clear_sub_task_errors()
    }

    pub fn get_my_uid(&self) -> Result<String> {
        Ok(self.sdk_api_client.session()?.uid.clone())
    }

    pub async fn get_username_by_id(&self, uid: i64) -> Result<Option<String>> {
        self.task_handler
            .get_user(uid)
            .await
            .map(|opt| opt.map(|u| u.screen_name))
    }

    pub async fn search_users_by_screen_name_prefix(&self, prefix: &str) -> Result<Vec<User>> {
        self.task_handler
            .search_users_by_screen_name_prefix(prefix)
            .await
    }

    // ========================= login stuff =========================

    pub async fn get_sms_code(&self, phone_number: String) -> Result<()> {
        info!("send_code called for phone number: {phone_number}");
        self.sdk_api_client.get_sms_code(phone_number).await?;
        Ok(())
    }

    pub async fn login(&self, sms_code: String) -> Result<User> {
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
                let ctx = self.create_short_task_context();
                let user_clone = user.clone();
                spawn(async move {
                    if let Err(e) = th.save_user_info(ctx, &user_clone).await {
                        error!("Save user info failed: {e}");
                    }
                });
                info!("Logged in user {} saved.", user_id);

                Ok(user)
            }
            LoginState::LoggedIn { .. } => {
                warn!("Already logged in, skipping login.");
                let session = self.sdk_api_client.session()?;
                let user: User = serde_json::from_value(session.user)?;
                Ok(user)
            }
            LoginState::Init => {
                error!("Wrong login state to login: Init");
                Err(crate::error::Error::InconsistentTask(
                    "FATAL: wrong login state to login".to_string(),
                ))
            }
        }
    }

    pub async fn login_state(&self) -> Result<Option<User>> {
        info!("get login state");
        Ok(self
            .sdk_api_client
            .session()
            .ok()
            .map(|s| serde_json::from_value(s.user))
            .transpose()?)
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

    // ========================= short tasks =========================

    pub async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPostInfo> {
        let ctx = self.create_short_task_context();
        self.task_handler.query_posts(ctx, query).await
    }

    pub async fn delete_post(&self, id: i64) -> Result<()> {
        let ctx = self.create_short_task_context();
        self.task_handler.delete_post(ctx, id).await
    }

    pub async fn rebackup_post(&self, id: i64) -> Result<()> {
        let ctx = self.create_short_task_context();
        self.task_handler.rebackup_post(ctx, id).await
    }

    pub async fn get_picture_blob(&self, id: &str) -> Result<Option<Bytes>> {
        let ctx = self.create_short_task_context();
        self.task_handler.get_picture_blob(ctx, id).await
    }

    // ========================= long tasks =========================

    pub async fn backup_user(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = request.total() as u64;
        self.task_manager
            .start_task(id, TaskType::BackupUser, "备份用户微博".into(), total)?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    pub async fn backup_favorites(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = request.total() as u64;
        self.task_manager
            .start_task(id, TaskType::BackupFavorites, "备份收藏".into(), total)?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    pub async fn unfavorite_posts(&self) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = 0; // Will be updated later in task_handler
        self.task_manager
            .start_task(id, TaskType::UnfavoritePosts, "取消收藏".into(), total)?;
        spawn(handle_task_request(
            self.task_handler.clone(),
            ctx,
            TaskRequest::UnfavoritePosts,
        ));
        Ok(())
    }

    pub async fn export_posts(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = request.total() as u64;
        self.task_manager
            .start_task(id, TaskType::Export, "导出微博".into(), total)?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    // ========================= context creators =========================

    fn create_long_task_context(&self) -> Arc<TaskContext> {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        Arc::new(TaskContext {
            task_id: Some(id),
            config: get_config().read().unwrap().clone(),
            task_manager: self.task_manager.clone(),
        })
    }

    fn create_short_task_context(&self) -> Arc<TaskContext> {
        Arc::new(TaskContext {
            task_id: None,
            config: get_config().read().unwrap().clone(),
            task_manager: self.task_manager.clone(),
        })
    }
}

async fn handle_task_request(task_handler: Arc<TH>, ctx: Arc<TaskContext>, request: TaskRequest) {
    let task_id = ctx.task_id.unwrap();
    info!("Handling task request for task_id: {}", task_id);
    debug!("Task request details: {:?}", request);

    let res = match request {
        TaskRequest::BackupUser(options) => task_handler.backup_user(ctx.clone(), options).await,
        TaskRequest::UnfavoritePosts => task_handler.unfavorite_posts(ctx.clone()).await,
        TaskRequest::BackupFavorites(options) => {
            task_handler.backup_favorites(ctx.clone(), options).await
        }
        TaskRequest::Export(options) => task_handler.export_posts(ctx.clone(), options).await,
    };

    if let Err(err) = res {
        error!("Task {} failed: {}", task_id, err);
        if let Err(e) = ctx.task_manager.fail(err.to_string()) {
            error!("Failed to set task {} as failed: {}", task_id, e);
        }
    } else {
        info!("Task {} completed successfully", task_id);
        if let Err(e) = ctx.task_manager.finish() {
            error!("Failed to set task {} as finished: {}", task_id, e);
        }
    }
}
