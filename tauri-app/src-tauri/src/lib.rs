mod error;

use std::sync::Arc;

use log::{error, info};
use serde::Serialize;
use serde_json::Value;
use tauri::{self, App, Manager, State};
use weiback::builder::CoreBuilder;
use weiback::config::{Config, get_config};
use weiback::core::{
    BackupFavoritesOptions, BackupUserPostsOptions, Core, ExportJobOptions, PostQuery, TaskRequest,
    task::{BackupType, PaginatedPostInfo},
    task_manager::{SubTaskError, Task},
};

use error::Result;
use tauri::ipc::Response;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum PictureError {
    NotFound,
    Internal(String),
}

#[tauri::command(async)]
async fn get_current_task_status(
    core: State<'_, Arc<Core>>,
) -> std::result::Result<Option<Task>, String> {
    core.get_current_task().await.map_err(|e| e.to_string())
}

#[tauri::command(async)]
async fn get_and_clear_sub_task_errors(
    core: State<'_, Arc<Core>>,
) -> std::result::Result<Vec<SubTaskError>, String> {
    core.get_and_clear_sub_task_errors()
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
async fn get_picture_blob(
    core: State<'_, Arc<Core>>,
    id: String,
) -> std::result::Result<Response, PictureError> {
    info!("get_picture_blob called, id: {id}");
    match core.get_picture_blob(id).await {
        Ok(Some(blob)) => Ok(Response::new(blob.to_vec())),
        Ok(None) => Err(PictureError::NotFound),
        Err(e) => Err(PictureError::Internal(e.to_string())),
    }
}

#[tauri::command]
fn get_config_command() -> std::result::Result<Config, String> {
    get_config()
        .read()
        .map(|guard| guard.clone())
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn set_config_command(config: Config) -> std::result::Result<(), String> {
    weiback::config::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn backup_user(
    core: State<'_, Arc<Core>>,
    uid: String,
    num_pages: u32,
    backup_type: BackupType,
) -> Result<()> {
    info!(
        "backup_user called with uid: {uid}, pages num: {num_pages}, backup_type: {backup_type:?}"
    );
    Ok(core
        .backup_user(TaskRequest::BackupUser(BackupUserPostsOptions {
            uid: uid.parse().map_err(|err| {
                error!("Failed to parse uid: {err}");
                err
            })?,
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
async fn login(core: State<'_, Arc<Core>>, sms_code: String) -> Result<()> {
    info!("login called with sms code: {sms_code}");
    Ok(core.login(sms_code).await?)
}

#[tauri::command]
async fn login_state(core: State<'_, Arc<Core>>) -> Result<Option<Value>> {
    info!("login_state called");
    Ok(core.login_state().await?)
}

#[tauri::command]
async fn delete_post(core: State<'_, Arc<Core>>, id: String) -> Result<()> {
    info!("delete_post called with id: {id}");
    let id = id.parse::<i64>().map_err(|err| {
        error!("Failed to parse id: {err}");
        err
    })?;
    Ok(core.delete_post(id).await?)
}

#[tauri::command]
async fn rebackup_post(core: State<'_, Arc<Core>>, id: String) -> Result<()> {
    info!("rebackup_post called with id: {id}");
    let id = id.parse::<i64>().map_err(|err| {
        error!("Failed to parse id: {err}");
        err
    })?;
    Ok(core.rebackup_post(id).await?)
}

#[tauri::command]
async fn get_username_by_id(
    core: State<'_, Arc<Core>>,
    uid: String,
) -> std::result::Result<Option<String>, String> {
    let Ok(uid) = uid.parse::<i64>() else {
        return Ok(None);
    };
    core.get_username_by_id(uid)
        .await
        .map_err(|e| e.to_string())
}

pub fn run() -> Result<()> {
    info!("Starting application");
    weiback::config::init()?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
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
            get_picture_blob,
            delete_post,
            rebackup_post,
            get_current_task_status,
            get_and_clear_sub_task_errors
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
    let core = CoreBuilder::new().build()?;
    info!("Setting up Tauri application");

    let core_clone = core.clone();
    tauri::async_runtime::spawn(async move { core_clone.login_with_session().await });

    app.manage(core);

    info!("Tauri setup complete");
    Ok(())
}
