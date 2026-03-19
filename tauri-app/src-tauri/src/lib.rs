mod error;

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{self, App, AppHandle, Emitter, Manager, State, ipc::Response};
use tracing::{debug, error, info, warn};
use weiback::builder::CoreBuilder;
use weiback::config::{Config, get_config};
use weiback::core::{
    BackupFavoritesOptions, BackupUserPostsOptions, CleanupInvalidPostsOptions, Core,
    ExportJobOptions, PostQuery, TaskEventListener, TaskRequest,
    task::{BackupType, CleanupPicturesOptions, PaginatedPostInfo},
    task_manager::{Task, TaskError},
};
use weiback::models::User;

use error::{Error, Result};

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "status")]
pub enum BackendStatus {
    Uninitialized,
    Running {
        #[serde(skip_serializing_if = "Option::is_none")]
        warning: Option<String>,
    },
    Error {
        message: String,
    },
}

pub struct BackendState {
    pub status: Mutex<BackendStatus>,
}

/// A reporter that forwards task events to the Tauri frontend via `emit`.
struct TauriTaskEventListener {
    app_handle: tauri::AppHandle,
}

impl TaskEventListener for TauriTaskEventListener {
    fn on_task_updated(&self, task: &Task) {
        debug!("emit task-updated to frontend: {task:?}");
        let _ = self.app_handle.emit("task-updated", task);
    }

    fn on_task_error(&self, error: &TaskError) {
        debug!("emit task-error to frontend: {error:?}");
        let _ = self.app_handle.emit("task-error", error);
    }
}

/// A wrapper for Weibo IDs to handle conversion from string/number in Tauri commands.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WeiboId(i64);

impl<'de> serde::Deserialize<'de> for WeiboId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<i64>()
            .map(WeiboId)
            .map_err(serde::de::Error::custom)
    }
}

impl From<WeiboId> for i64 {
    fn from(id: WeiboId) -> Self {
        id.0
    }
}

#[tauri::command]
async fn get_backend_status(state: State<'_, BackendState>) -> Result<BackendStatus> {
    Ok(state.status.lock().unwrap().clone())
}

fn perform_init_backend(app_handle: &AppHandle, state: &BackendState) -> BackendStatus {
    let mut status_guard = state.status.lock().unwrap();
    if let BackendStatus::Running { .. } = *status_guard {
        return status_guard.clone();
    }

    info!("Initializing backend core...");
    // Attempt to initialize config from files.
    let mut warning = None;
    if let Err(e) = weiback::config::init() {
        warn!("Config initialization failed, using default: {e}");
        warning = Some(e.to_string());
        // Fallback to in-memory default configuration.
        weiback::config::init_default();
    }

    match CoreBuilder::new().build() {
        Ok(core) => {
            let listener = Box::new(TauriTaskEventListener {
                app_handle: app_handle.clone(),
            });
            if let Err(e) = core.set_task_event_listener(listener) {
                error!("Failed to set task event listener: {e}");
            }

            let core_clone = core.clone();
            tauri::async_runtime::spawn(async move { core_clone.login_with_session().await });

            app_handle.manage(core);
            *status_guard = BackendStatus::Running { warning };
            info!("Backend initialized successfully");
            status_guard.clone()
        }
        Err(e) => {
            error!("Backend initialization failed: {e}");
            *status_guard = BackendStatus::Error {
                message: e.to_string(),
            };
            status_guard.clone()
        }
    }
}

#[tauri::command]
fn init_backend(app_handle: AppHandle, state: State<'_, BackendState>) -> Result<BackendStatus> {
    Ok(perform_init_backend(&app_handle, &state))
}

#[tauri::command(async)]
async fn get_current_task_status(core: State<'_, Arc<Core>>) -> Result<Option<Task>> {
    core.get_current_task()
        .await
        .map_err(|e| Error(e.to_string()))
}

#[tauri::command(async)]
async fn get_and_clear_task_errors(core: State<'_, Arc<Core>>) -> Result<Vec<TaskError>> {
    core.get_and_clear_task_errors()
        .map_err(|e| Error(e.to_string()))
}

#[tauri::command(async)]
async fn get_picture_blob(core: State<'_, Arc<Core>>, id: String) -> Result<Response> {
    match core.get_picture_blob(&id).await {
        Ok(Some(blob)) => {
            debug!("get_picture_blob called, id: {id}");
            Ok(Response::new(blob.to_vec()))
        }
        Ok(None) => {
            warn!("get_picture_blob called: {id} not found");
            Err(Error("Picture not found".to_string()))
        }
        Err(e) => {
            error!("get_picture_blob called: {e:?}");
            Err(Error(e.to_string()))
        }
    }
}

#[tauri::command(async)]
async fn get_video_blob(core: State<'_, Arc<Core>>, url: String) -> Result<Response> {
    match core.get_video_blob(&url).await {
        Ok(Some(blob)) => {
            debug!("get_video_blob called, url: {url}");
            Ok(Response::new(blob.to_vec()))
        }
        Ok(None) => {
            warn!("get_video_blob called: {url} not found");
            Err(Error("Video not found".to_string()))
        }
        Err(e) => {
            error!("get_video_blob called: {e:?}");
            Err(Error(e.to_string()))
        }
    }
}

#[tauri::command]
fn get_config_command() -> Result<Config> {
    get_config()
        .read()
        .map(|guard| guard.clone())
        .map_err(|err| Error(err.to_string()))
}

#[tauri::command]
fn set_config_command(config: Config) -> Result<()> {
    weiback::config::save_config(&config).map_err(|e| Error(e.to_string()))
}

#[tauri::command]
async fn backup_user(
    core: State<'_, Arc<Core>>,
    uid: WeiboId,
    num_pages: u32,
    backup_type: BackupType,
) -> Result<()> {
    info!(
        "backup_user called with uid: {:?}, pages num: {num_pages}, backup_type: {backup_type:?}",
        uid
    );
    Ok(core
        .backup_user(TaskRequest::BackupUser(BackupUserPostsOptions {
            uid: uid.into(),
            num_pages,
            backup_type,
        }))
        .await?)
}

#[tauri::command]
async fn backup_favorites(core: State<'_, Arc<Core>>, num_pages: u32) -> Result<()> {
    info!("backup_favorites called with pages num: {num_pages}");
    Ok(core
        .backup_favorites(TaskRequest::BackupFavorites(BackupFavoritesOptions {
            num_pages,
        }))
        .await?)
}

#[tauri::command]
async fn unfavorite_posts(core: State<'_, Arc<Core>>) -> Result<()> {
    info!("unfavorite_posts called");
    Ok(core.unfavorite_posts().await?)
}

#[tauri::command]
async fn export_posts(core: State<'_, Arc<Core>>, options: ExportJobOptions) -> Result<()> {
    info!("export_from_local called with options: {options:?}");
    Ok(core.export_posts(TaskRequest::Export(options)).await?)
}

#[tauri::command]
async fn query_local_posts(
    core: State<'_, Arc<Core>>,
    query: PostQuery,
) -> Result<PaginatedPostInfo> {
    info!("query_local_posts called with query: {query:?}");
    Ok(core.query_posts(query).await?)
}

#[tauri::command]
async fn get_sms_code(core: State<'_, Arc<Core>>, phone_number: String) -> Result<()> {
    info!("get_sms_code called with phone number: {phone_number}");
    Ok(core.get_sms_code(phone_number).await?)
}

#[tauri::command]
async fn login(core: State<'_, Arc<Core>>, sms_code: String) -> Result<User> {
    info!("login called with sms code: {sms_code}");
    Ok(core.login(sms_code).await?)
}

#[tauri::command]
async fn login_state(core: State<'_, Arc<Core>>) -> Result<Option<User>> {
    info!("login_state called");
    Ok(core.login_state().await?)
}

#[tauri::command]
async fn delete_post(core: State<'_, Arc<Core>>, id: WeiboId) -> Result<()> {
    info!("delete_post called with id: {id:?}");
    Ok(core.delete_post(id.into()).await?)
}

#[tauri::command]
async fn rebackup_post(core: State<'_, Arc<Core>>, id: WeiboId) -> Result<()> {
    info!("rebackup_post called with id: {id:?}");
    Ok(core.rebackup_post(id.into()).await?)
}

#[tauri::command]
async fn rebackup_posts(core: State<'_, Arc<Core>>, query: PostQuery) -> Result<()> {
    info!("rebackup_posts called with query: {query:?}");
    Ok(core
        .rebackup_posts(TaskRequest::RebackupPosts(query))
        .await?)
}

#[tauri::command]
async fn rebackup_missing_images(core: State<'_, Arc<Core>>, query: PostQuery) -> Result<()> {
    info!("rebackup_missing_images called with query: {query:?}");
    Ok(core
        .rebackup_missing_images(TaskRequest::RebackupMissingImages(query))
        .await?)
}

#[tauri::command]
async fn get_username_by_id(core: State<'_, Arc<Core>>, uid: WeiboId) -> Result<Option<String>> {
    core.get_username_by_id(uid.into())
        .await
        .map_err(|e| Error(e.to_string()))
}

#[tauri::command(async)]
async fn search_id_by_username_prefix(
    core: State<'_, Arc<Core>>,
    prefix: String,
) -> Result<Vec<User>> {
    info!("search_id_by_username_prefix called with prefix: {prefix}");
    core.search_users_by_screen_name_prefix(&prefix)
        .await
        .map_err(|e| Error(e.to_string()))
}

#[tauri::command]
async fn cleanup_pictures(
    core: State<'_, Arc<Core>>,
    options: CleanupPicturesOptions,
) -> Result<()> {
    info!("cleanup_pictures called with options: {options:?}");
    Ok(core
        .cleanup_pictures(TaskRequest::CleanupPictures(options))
        .await?)
}

#[tauri::command]
async fn cleanup_invalid_avatars(core: State<'_, Arc<Core>>) -> Result<()> {
    info!("cleanup_invalid_avatars called");
    Ok(core.cleanup_invalid_avatars().await?)
}

#[tauri::command]
async fn cleanup_invalid_posts(
    core: State<'_, Arc<Core>>,
    options: CleanupInvalidPostsOptions,
) -> Result<()> {
    info!("cleanup_invalid_posts called with options: {options:?}");
    Ok(core
        .cleanup_invalid_posts(TaskRequest::CleanupInvalidPosts(options))
        .await?)
}

#[tauri::command]
async fn cleanup_invalid_pictures(core: State<'_, Arc<Core>>) -> Result<()> {
    info!("cleanup_invalid_pictures called");
    Ok(core
        .cleanup_invalid_pictures(TaskRequest::CleanupInvalidPictures)
        .await?)
}

pub fn run() -> Result<()> {
    info!("Starting application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            get_backend_status,
            init_backend,
            backup_user,
            backup_favorites,
            unfavorite_posts,
            export_posts,
            query_local_posts,
            get_sms_code,
            login,
            login_state,
            get_config_command,
            set_config_command,
            get_username_by_id,
            search_id_by_username_prefix,
            get_picture_blob,
            get_video_blob,
            delete_post,
            rebackup_post,
            rebackup_posts,
            rebackup_missing_images,
            get_current_task_status,
            get_and_clear_task_errors,
            cleanup_pictures,
            cleanup_invalid_avatars,
            cleanup_invalid_posts,
            cleanup_invalid_pictures
        ])
        .build(tauri::generate_context!())
        .expect("tauri app build failed")
        .run(|_app_handle, _event| {
            #[cfg(feature = "dev-mode")]
            if let tauri::RunEvent::ExitRequested { code, api, .. } = _event
                && code.is_none()
            {
                api.prevent_exit();
                weiback::dev_client::save_records();
                _app_handle.cleanup_before_exit();
                _app_handle.exit(0);
            }
        });
    Ok(())
}

fn setup(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    info!("Setting up Tauri application state");
    let state = BackendState {
        status: Mutex::new(BackendStatus::Uninitialized),
    };

    app.manage(state);
    info!("Tauri setup complete");
    Ok(())
}
