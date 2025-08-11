use std::ops::RangeInclusive;
use std::sync::Arc;

use log::{debug, error, info, warn};
use tauri::{self, AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, mpsc};
use weiback::config::get_config;
use weiback::core::{
    BFOptions, BUOptions, Core, TaskRequest, task_handler::TaskHandler, task_manager::TaskManger,
};
use weiback::error::{Error, Result};
use weiback::exporter::{ExportOptions, ExporterImpl};
use weiback::media_downloader::MediaDownloaderImpl;
use weiback::message::{ErrMsg, Message, TaskProgress};
use weiback::storage::StorageImpl;
use weibosdk_rs::{WeiboAPIImpl as WAI, client::new_client_with_headers, weibo_api::LoginState};

type TH = TaskHandler<WeiboAPIImpl, Arc<StorageImpl>, ExporterImpl, MediaDownloaderImpl>;
type WeiboAPIImpl = WAI<reqwest::Client>;

#[tauri::command]
async fn backup_self(
    core: State<'_, Mutex<Core>>,
    api_client: State<'_, Mutex<WeiboAPIImpl>>,
    range: RangeInclusive<u32>,
) -> Result<()> {
    info!("backup_self called with range: {range:?}");
    let uid = api_client.lock().await.session()?.uid.clone();
    backup_user(core, uid, range).await
}

#[tauri::command]
async fn backup_user(
    core: State<'_, Mutex<Core>>,
    uid: String,
    range: RangeInclusive<u32>,
) -> Result<()> {
    info!("backup_user called with uid: {uid}, range: {range:?}");
    core.lock()
        .await
        .backup_user(TaskRequest::BackupUser(BUOptions {
            uid: uid.parse().map_err(|err| {
                error!("Failed to parse uid: {err}");
                Error::Other(format!("{err}"))
            })?,
            range,
        }))
        .await
}

#[tauri::command]
async fn backup_favorites(core: State<'_, Mutex<Core>>, range: RangeInclusive<u32>) -> Result<()> {
    info!("backup_favorites called with range: {range:?}");
    core.lock()
        .await
        .backup_favorites(TaskRequest::BackupFavorites(BFOptions { range }))
        .await
}

#[tauri::command]
async fn unfavorite_posts(core: State<'_, Mutex<Core>>) -> Result<()> {
    info!("unfavorite_posts called");
    core.lock().await.unfavorite_posts().await
}

#[tauri::command]
async fn export_from_local(task_handler: State<'_, TH>, range: RangeInclusive<u32>) -> Result<()> {
    info!("export_from_local called with range: {range:?}");
    let options = ExportOptions::new().range(range);
    task_handler.export_from_local(options).await
}

#[tauri::command]
async fn send_code(api_client: State<'_, Mutex<WeiboAPIImpl>>, phone_number: String) -> Result<()> {
    info!(
        "send_code called for phone number (partially hidden): ...{}",
        &phone_number.chars().skip(7).collect::<String>()
    );
    let mut api_client = api_client.lock().await;
    match api_client.login_state() {
        LoginState::LoggedIn { .. } => {
            warn!("Already logged in, skipping send_code.");
            Ok(())
        }
        _ => {
            info!("Sending SMS code.");
            api_client.get_sms_code(phone_number).await?;
            Ok(())
        }
    }
}

#[tauri::command]
async fn login(api_client: State<'_, Mutex<WeiboAPIImpl>>, sms_code: String) -> Result<()> {
    info!("login called with a sms_code");
    let mut api_client = api_client.lock().await;
    match api_client.login_state() {
        LoginState::WaitingForCode { .. } => {
            info!("Attempting to login with SMS code.");
            api_client.login(&sms_code).await?;
            info!("Login successful.");
            Ok(())
        }
        LoginState::LoggedIn { .. } => {
            warn!("Already logged in, skipping login.");
            Ok(())
        }
        LoginState::Init => {
            error!("Wrong login state to login: Init");
            Err(Error::Other(
                "FATAL: wrong login state to login".to_string(),
            ))
        }
    }
}

pub fn run() -> Result<()> {
    env_logger::init();
    info!("Starting application");

    let weibo_api_config = get_config()
        .read()
        .map_err(|e| {
            error!("Failed to read config: {e}");
            Error::Other(e.to_string())
        })?
        .weibo_api_config
        .clone();
    debug!("Weibo API config loaded: {weibo_api_config:?}");

    let (msg_sender, msg_receiver) = mpsc::channel(100);
    info!("MPSC channel created");

    let storage = StorageImpl::new().unwrap(); // TODO: handle error
    let storage = Arc::new(storage);
    info!("Storage initialized");

    let exporter = ExporterImpl::new();
    info!("Exporter initialized");

    let http_client = new_client_with_headers().unwrap(); // TODO: handle error
    info!("HTTP client created");

    let downloader = MediaDownloaderImpl::new(http_client.clone(), msg_sender.clone());
    info!("MediaDownloader initialized");

    let api_client = WeiboAPIImpl::new(http_client.clone(), weibo_api_config);
    info!("WeiboAPIImpl initialized");

    let task_handler = TaskHandler::new(
        api_client.clone(),
        storage,
        exporter,
        downloader,
        msg_sender,
    )
    .unwrap(); // TODO: handle error
    info!("TaskHandler initialized");

    let core = Core::new(task_handler.clone())?;
    info!("Core initialized");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            info!("Setting up Tauri application");
            tauri::async_runtime::spawn(msg_loop(
                app.handle().clone(),
                core.task_manager().clone(),
                msg_receiver,
            ));
            app.manage(Mutex::new(core));
            app.manage(task_handler);
            app.manage(Mutex::new(api_client));
            info!("Tauri setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backup_user,
            backup_self,
            backup_favorites,
            unfavorite_posts,
            export_from_local,
            send_code,
            login
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application"); // TODO
    Ok(())
}

async fn msg_loop(
    app: AppHandle,
    task_manager: TaskManger,
    mut msg_receiver: mpsc::Receiver<Message>,
) {
    info!("Message loop started");
    loop {
        tokio::select! {
            Some(msg) = msg_receiver.recv() => {
                debug!("Received message: {msg:?}");
                if let Err(e) = handle_task_responses(&app, &task_manager, msg).await {
                    error!("Error handling task response: {e}");
                }
            }
            else => {
                warn!("Message channel closed, exiting message loop.");
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
            debug!("Handling TaskProgress message for task_id: {task_id}");
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
            error!("Handling ErrMsg for task_id: {task_id}, type: {type:?}, error: {err}",);
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
