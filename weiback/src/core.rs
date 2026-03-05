//! The `core` module is the heart of the application, coordinating task execution,
//! user session management, and high-level operations.
//!
//! It provides the [`Core`] struct, which serves as the primary interface for the
//! frontend (via Tauri or CLI) to trigger actions like backing up posts, exporting
//! data, and managing login states.
//!
//! Key components within this module include:
//! - [`TaskHandler`]: Implements the specific logic for various backup and export tasks.
//! - [`TaskManager`]: Tracks the status and progress of currently running tasks.
//! - [`PostProcesser`]: Handles the downloading of media and insertion of posts into storage.

pub mod post_processer;
pub mod task;
pub mod task_handler;
pub mod task_manager;

use bytes::Bytes;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::spawn;
use tracing::{debug, error, info, warn};
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
    BackupFavoritesOptions, BackupUserPostsOptions, CleanupInvalidPostsOptions, ExportJobOptions,
    PaginatedPostInfo, PostQuery, TaskContext, TaskRequest, UserPostFilter,
};
pub use task_handler::TaskHandler;
pub use task_manager::{SubTaskError, Task, TaskManager, TaskType};

#[cfg(not(feature = "dev-mode"))]
type TH = TaskHandler<DefaultApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;
#[cfg(feature = "dev-mode")]
type TH = TaskHandler<DevApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;

#[cfg(feature = "dev-mode")]
type CurrentSdkApiClient = SdkApiClient<crate::dev_client::DevClient>;
#[cfg(not(feature = "dev-mode"))]
type CurrentSdkApiClient = SdkApiClient<weibosdk_rs::Client>;

/// The main application engine that orchestrates all services.
///
/// `Core` maintains the state of running tasks and provides high-level methods for
/// interacting with Weibo APIs and local storage. It is typically wrapped in an
/// [`Arc`] and shared across the application.
pub struct Core {
    next_task_id: AtomicU64,
    task_handler: Arc<TH>,
    task_manager: Arc<TaskManager>,
    sdk_api_client: Arc<CurrentSdkApiClient>,
}

impl Core {
    /// Creates a new `Core` instance.
    ///
    /// This is an internal constructor used by `CoreBuilder`.
    pub(crate) fn new(task_handler: TH, sdk_api_client: Arc<CurrentSdkApiClient>) -> Result<Self> {
        Ok(Self {
            next_task_id: AtomicU64::new(1),
            task_handler: Arc::new(task_handler),
            task_manager: Arc::new(TaskManager::new()),
            sdk_api_client,
        })
    }

    /// Retrieves the status of the currently active long-running task.
    ///
    /// # Returns
    /// A `Result` containing `Some(Task)` if a task is running or recently finished, or `None`.
    pub async fn get_current_task(&self) -> Result<Option<Task>> {
        self.task_manager.get_current()
    }

    /// Collects and removes all non-fatal sub-task errors (e.g., download failures).
    ///
    /// This should be called periodically by the UI to report issues to the user.
    pub fn get_and_clear_sub_task_errors(&self) -> Result<Vec<SubTaskError>> {
        self.task_manager.get_and_clear_sub_task_errors()
    }

    /// Gets the Weibo UID of the currently logged-in user.
    ///
    /// # Errors
    /// Returns an error if no active session is found.
    pub fn get_my_uid(&self) -> Result<String> {
        Ok(self.sdk_api_client.session()?.uid.clone())
    }

    /// Retrieves a user's screen name from local storage by their UID.
    ///
    /// # Arguments
    /// * `uid` - The unique identifier of the user.
    pub async fn get_username_by_id(&self, uid: i64) -> Result<Option<String>> {
        self.task_handler
            .get_user(uid)
            .await
            .map(|opt| opt.map(|u| u.screen_name))
    }

    /// Searches for users in local storage whose screen name starts with the given prefix.
    pub async fn search_users_by_screen_name_prefix(&self, prefix: &str) -> Result<Vec<User>> {
        self.task_handler
            .search_users_by_screen_name_prefix(prefix)
            .await
    }

    // ========================= login stuff =========================

    /// Requests an SMS login code for the specified phone number.
    ///
    /// # Arguments
    /// * `phone_number` - The phone number to send the code to (e.g., "13800138000").
    pub async fn get_sms_code(&self, phone_number: String) -> Result<()> {
        info!("send_code called for phone number: {phone_number}");
        self.sdk_api_client.get_sms_code(phone_number).await?;
        Ok(())
    }

    /// Completes the login process using an SMS code.
    ///
    /// This method updates the session, saves it to disk, and persists the logged-in
    /// user's information to local storage.
    ///
    /// # Arguments
    /// * `sms_code` - The code received via SMS.
    ///
    /// # Errors
    /// Returns an error if the login fails or if the system is not in the `WaitingForCode` state.
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

    /// Checks the current login state and returns the logged-in user if available.
    pub async fn login_state(&self) -> Result<Option<User>> {
        info!("get login state");
        Ok(self
            .sdk_api_client
            .session()
            .ok()
            .map(|s| serde_json::from_value(s.user))
            .transpose()?)
    }

    /// Attempts to restore a session from the saved session file.
    ///
    /// Useful for persisting login across application restarts.
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

    /// Queries local posts based on the provided search and filter criteria.
    pub async fn query_posts(&self, query: PostQuery) -> Result<PaginatedPostInfo> {
        let ctx = self.create_short_task_context();
        self.task_handler.query_posts(ctx, query).await
    }

    /// Deletes a post from local storage.
    pub async fn delete_post(&self, id: i64) -> Result<()> {
        let ctx = self.create_short_task_context();
        self.task_handler.delete_post(ctx, id).await
    }

    /// Re-fetches a single post from the Weibo API and updates local storage.
    pub async fn rebackup_post(&self, id: i64) -> Result<()> {
        let ctx = self.create_short_task_context();
        self.task_handler.rebackup_post(ctx, id).await
    }

    /// Retrieves the raw image data (blob) for a given picture ID.
    pub async fn get_picture_blob(&self, id: &str) -> Result<Option<Bytes>> {
        let ctx = self.create_short_task_context();
        self.task_handler.get_picture_blob(ctx, id).await
    }

    /// Export local posts to another format (e.g., HTML).
    pub async fn export_posts(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_short_task_context();
        if let TaskRequest::Export(options) = request {
            self.task_handler.export_posts(ctx, options).await
        } else {
            Err(crate::error::Error::InconsistentTask(
                "Invalid task request for export_posts".into(),
            ))
        }
    }

    /// Clean up redundant or low-resolution images.
    pub async fn cleanup_pictures(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_short_task_context();
        if let TaskRequest::CleanupPictures(options) = request {
            self.task_handler.cleanup_pictures(ctx, options).await
        } else {
            Err(crate::error::Error::InconsistentTask(
                "Invalid task request for cleanup_pictures".into(),
            ))
        }
    }

    /// Clean up invalid or outdated avatars.
    pub async fn cleanup_invalid_avatars(&self) -> Result<()> {
        let ctx = self.create_short_task_context();
        self.task_handler.cleanup_invalid_avatars(ctx).await
    }

    /// Clean up invalid posts.
    pub async fn cleanup_invalid_posts(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_short_task_context();
        if let TaskRequest::CleanupInvalidPosts(options) = request {
            self.task_handler.cleanup_invalid_posts(ctx, options).await
        } else {
            Err(crate::error::Error::InconsistentTask(
                "Invalid task request for cleanup_invalid_posts".into(),
            ))
        }
    }

    // ========================= long tasks =========================

    /// Starts a long-running task to backup a user's posts.
    pub async fn backup_user(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = request.total() as u64;
        self.task_manager
            .start_task(id, TaskType::BackupUser, "备份用户微博".into(), total)?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    /// Starts a long-running task to backup the current user's favorites.
    pub async fn backup_favorites(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = request.total() as u64;
        self.task_manager
            .start_task(id, TaskType::BackupFavorites, "备份收藏".into(), total)?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    /// Starts a long-running task to unfavorite posts that are in the local database.
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

    /// Starts a long-running task to re-backup posts.
    pub async fn rebackup_posts(&self, request: TaskRequest) -> Result<()> {
        let ctx = self.create_long_task_context();
        let id = ctx.task_id.unwrap();
        let total = 0; // Will be updated in task_handler
        self.task_manager
            .start_task(id, TaskType::RebackupPosts, "批量重新备份".into(), total)?;
        spawn(handle_task_request(self.task_handler.clone(), ctx, request));
        Ok(())
    }

    // ========================= context creators =========================

    /// Creates a task context for long-running tasks, including a unique task ID.
    fn create_long_task_context(&self) -> Arc<TaskContext> {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        Arc::new(TaskContext {
            task_id: Some(id),
            config: get_config().read().unwrap().clone(),
            task_manager: self.task_manager.clone(),
        })
    }

    /// Creates a task context for short-lived operations that do not require progress tracking.
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
        TaskRequest::RebackupPosts(query) => task_handler.rebackup_posts(ctx.clone(), query).await,
        _ => {
            error!("Unexpected TaskRequest for long task: {:?}", request);
            Err(crate::error::Error::InconsistentTask(format!(
                "Unexpected TaskRequest: {:?}",
                request
            )))
        }
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
