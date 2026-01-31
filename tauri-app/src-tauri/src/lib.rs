mod error;

use std::sync::Arc;

use log::{error, info};
use serde_json::Value;
use tauri::{self, App, AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;
use weiback::builder::CoreBuilder;
use weiback::config::{Config, get_config};
use weiback::core::{
    BackupFavoritesOptions, BackupUserPostsOptions, Core, ExportJobOptions, PaginatedPosts,
    PostQuery, TaskRequest, task_manager::TaskManger,
};
use weiback::message::{ErrMsg, Message, TaskProgress};

use error::Result;

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
async fn backup_user(core: State<'_, Arc<Core>>, uid: String, num_pages: u32) -> Result<()> {
    info!("backup_user called with uid: {uid}, pages num: {num_pages}");
    Ok(core
        .backup_user(TaskRequest::BackupUser(BackupUserPostsOptions {
            uid: uid.parse().map_err(|err| {
                error!("Failed to parse uid: {err}");
                err
            })?,
            num_pages,
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
    Ok(core.export_posts(options).await?)
}

#[tauri::command]
async fn query_local_posts(core: State<'_, Arc<Core>>, query: PostQuery) -> Result<PaginatedPosts> {
    info!("query_local_posts called with query: {query:?}");
    Ok(core.query_local_posts(query).await?)
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
            get_username_by_id
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
    let (core, msg_receiver) = CoreBuilder::new().build()?;

    info!("Setting up Tauri application");
    tauri::async_runtime::spawn(msg_loop(
        app.handle().clone(),
        core.task_manager(),
        msg_receiver,
    ));

    let core_clone = core.clone();
    tauri::async_runtime::spawn(async move { core_clone.login_with_session().await });

    app.manage(core);

    info!("Tauri setup complete");
    Ok(())
}

async fn msg_loop(
    app: AppHandle,
    task_manager: Arc<TaskManger>,
    mut msg_receiver: mpsc::Receiver<Message>,
) {
    info!("Message loop started");
    loop {
        tokio::select! {
            Some(msg) = msg_receiver.recv() => {
                if let Err(e) = handle_task_responses(&app, &task_manager, msg).await {
                    error!("Error handling task response: {:?}", e);
                }
            }
            else => {
                info!("Message channel closed, exiting message loop.");
                break;
            }
        }
    }
}

async fn handle_task_responses(
    app: &AppHandle,
    task_manager: &TaskManger,
    msg: Message,
) -> Result<()> {
    match msg {
        Message::TaskProgress(TaskProgress {
            r#type,
            task_id,
            total_increment,
            progress_increment,
        }) => {
            let (progress_new, total_new) =
                task_manager.update_progress(task_id, progress_increment, total_increment)?;

            app.emit(
                "task-progress",
                serde_json::json!({
                    "type":r#type,
                    "total":total_new,
                    "progress":progress_new
                }),
            )?;
        }
        Message::Err(ErrMsg {
            task_id,
            err,
            r#type,
        }) => {
            error!("Handling ErrMsg for task_id: {task_id}, type: {type:?}, error: {err}");
            app.emit(
                "error",
                serde_json::json!({
                    "type": r#type,
                    "task_id":task_id,
                    "err": err.to_string(),
                }),
            )?
        }
    }
    Ok(())
}
