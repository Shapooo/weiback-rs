mod error;

use std::ops::RangeInclusive;

use log::{debug, error, info, warn};
use tauri::{self, App, AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, mpsc};
use weiback::config::get_config;
use weiback::core::{
    BFOptions, BUOptions, Core, ExportOptions, TaskRequest, task_handler::TaskHandler,
    task_manager::TaskManger,
};
use weiback::exporter::ExporterImpl;
use weiback::media_downloader::{MediaDownloaderHandle, create_downloader};
use weiback::message::{ErrMsg, Message, TaskProgress};
use weiback::storage::StorageImpl;
use weibosdk_rs::{Client, WeiboAPIImpl as WAI, weibo_api::LoginState};

use error::{Error, Result};

type TH = TaskHandler<WeiboAPIImpl, StorageImpl, ExporterImpl, MediaDownloaderHandle>;
type WeiboAPIImpl = WAI<Client>;

#[tauri::command]
async fn backup_self(
    core: State<'_, Core>,
    api_client: State<'_, Mutex<WeiboAPIImpl>>,
    range: RangeInclusive<u32>,
) -> Result<()> {
    info!("backup_self called with range: {range:?}");
    let uid = api_client.lock().await.session()?.uid.clone();
    backup_user(core, uid, range).await
}

#[tauri::command]
async fn backup_user(core: State<'_, Core>, uid: String, range: RangeInclusive<u32>) -> Result<()> {
    info!("backup_user called with uid: {uid}, range: {range:?}");
    Ok(core
        .backup_user(TaskRequest::BackupUser(BUOptions {
            uid: uid.parse().map_err(|err| {
                error!("Failed to parse uid: {err}");
                err
            })?,
            range,
        }))
        .await?)
}

#[tauri::command]
async fn backup_favorites(core: State<'_, Core>, range: RangeInclusive<u32>) -> Result<()> {
    info!("backup_favorites called with range: {range:?}");
    Ok(core
        .backup_favorites(TaskRequest::BackupFavorites(BFOptions { range }))
        .await?)
}

#[tauri::command]
async fn unfavorite_posts(core: State<'_, Core>) -> Result<()> {
    info!("unfavorite_posts called");
    Ok(core.unfavorite_posts().await?)
}

#[tauri::command]
async fn export_from_local(task_handler: State<'_, TH>, range: RangeInclusive<u32>) -> Result<()> {
    info!("export_from_local called with range: {range:?}");
    let options = ExportOptions {
        range,
        ..Default::default()
    };
    Ok(task_handler.export_from_local(options).await?)
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
            Err(Error("FATAL: wrong login state to login".to_string()))
        }
    }
}

pub fn run() -> Result<()> {
    info!("Starting application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(setup)
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
        .map_err(|e| {
            error!("{e}");
            e
        })?;
    Ok(())
}

fn setup(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let weibo_api_config = get_config()
        .read()
        .map_err(|e| {
            error!("Failed to read config: {e}");
            anyhow::anyhow!("{e}")
        })?
        .weibo_api_config
        .clone();
    debug!("Weibo API config loaded: {weibo_api_config:?}");

    let (msg_sender, msg_receiver) = mpsc::channel(100);
    info!("MPSC channel created");

    let storage = StorageImpl::new()?;
    info!("Storage initialized");

    let exporter = ExporterImpl::new();
    info!("Exporter initialized");

    let http_client = Client::new()?;
    info!("HTTP client created");

    let (handle, worker) =
        create_downloader(100, http_client.main_client().clone(), msg_sender.clone());
    tauri::async_runtime::spawn(worker.run());
    info!("MediaDownloader initialized");

    let api_client = WeiboAPIImpl::new(http_client.clone(), weibo_api_config);
    info!("WeiboAPIImpl initialized");

    let task_handler = TaskHandler::new(api_client.clone(), storage, exporter, handle, msg_sender)?;
    info!("TaskHandler initialized");

    let core = Core::new(task_handler.clone())?;
    info!("Core initialized");

    info!("Setting up Tauri application");

    tauri::async_runtime::spawn(msg_loop(
        app.handle().clone(),
        core.task_manager().clone(),
        msg_receiver,
    ));
    app.manage(core);
    app.manage(task_handler);
    app.manage(Mutex::new(api_client.clone()));

    if let Ok(session) = Session::load(config.session_path.as_path()) {
        tauri::async_runtime::spawn_blocking(async move || {
            if let Err(e) = api_client.clone().login_with_session(session).await {
                error!("{e}");
            }
        });
    }

    info!("Tauri setup complete");
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
                    error!("Error handling task response: {e:?}");
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
