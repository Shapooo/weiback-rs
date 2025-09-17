mod error;
mod status_manager;

use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};

use log::{debug, error, info, warn};
use tauri::{self, App, AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;

use weiback::api::ApiClientImpl;
use weiback::config::get_config;
use weiback::core::{
    BFOptions, BUOptions, Core, ExportOptions, TaskRequest, task_handler::TaskHandler,
    task_manager::TaskManger,
};
use weiback::exporter::ExporterImpl;
use weiback::media_downloader::{MediaDownloaderHandle, create_downloader};
use weiback::message::{ErrMsg, Message, TaskProgress};
use weiback::storage::StorageImpl;
use weibosdk_rs::{
    ApiClient as SdkApiClient, Client as HttpClient, api_client::LoginState, session::Session,
};

use error::{Error, Result};
use status_manager::StatusManager;

#[cfg(feature = "dev-mode")]
type TH = TaskHandler<weiback::api::DevApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;
#[cfg(not(feature = "dev-mode"))]
type TH =
    TaskHandler<weiback::api::DefaultApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;

#[tauri::command]
async fn backup_self(
    core: State<'_, Core>,
    api_client: State<'_, Mutex<SdkApiClient<HttpClient>>>,
    range: RangeInclusive<u32>,
) -> Result<()> {
    info!("backup_self called with range: {range:?}");
    let uid = api_client
        .lock()
        .unwrap()
        .session()?
        .lock()
        .unwrap()
        .uid
        .clone();
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
async fn send_code(
    api_client: State<'_, tokio::sync::Mutex<SdkApiClient<HttpClient>>>,
    phone_number: String,
) -> Result<()> {
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
async fn login(
    api_client: State<'_, tokio::sync::Mutex<SdkApiClient<HttpClient>>>,
    status_manager: State<'_, StatusManager>,
    sms_code: String,
) -> Result<()> {
    info!("login called with a sms_code");
    let mut api_client = api_client.lock().await;
    match api_client.login_state() {
        LoginState::WaitingForCode { .. } => {
            info!("Attempting to login with SMS code.");
            api_client.login(&sms_code).await?;
            if let LoginState::LoggedIn { session } = api_client.login_state() {
                status_manager.set_session(session.clone());
            }
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

#[tauri::command]
fn login_status(status_manager: State<'_, StatusManager>) -> Result<Option<String>> {
    info!("get login status");
    Ok(status_manager
        .session()
        .map(|s| s.lock().unwrap().uid.clone()))
}

pub fn run() -> Result<()> {
    info!("Starting application");
    weiback::config::init()?;

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
            login,
            login_status
        ])
        .run(tauri::generate_context!())
        .map_err(|e| {
            error!("{e}");
            e
        })?;
    Ok(())
}

fn setup(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let main_config = get_config();
    let main_config = main_config.read().map_err(|e| {
        error!("Failed to read config: {e}");
        anyhow::anyhow!("{e}")
    })?;
    debug!("Weibo API config loaded: {main_config:?}");

    let (msg_sender, msg_receiver) = mpsc::channel(100);
    info!("MPSC channel created");

    let storage = StorageImpl::new()?;
    info!("Storage initialized");

    let exporter = ExporterImpl::new();
    info!("Exporter initialized");

    let http_client = HttpClient::new()?;
    info!("HTTP client created");

    let (handle, worker) =
        create_downloader(100, http_client.main_client().clone(), msg_sender.clone());
    tauri::async_runtime::spawn(worker.run());
    info!("MediaDownloader initialized");

    #[cfg(feature = "dev-mode")]
    let dev_client = weiback::dev_client::DevClient::new(
        http_client.clone(),
        main_config.dev_mode_out_dir.clone(),
    );
    #[cfg(feature = "dev-mode")]
    let sdk_api_client = SdkApiClient::new(dev_client, main_config.sdk_config.clone());
    #[cfg(not(feature = "dev-mode"))]
    let sdk_api_client = SdkApiClient::new(http_client.clone(), main_config.sdk_config.clone());
    info!("WeiboAPIImpl initialized");

    let api_client = ApiClientImpl::new(sdk_api_client.clone());

    let task_handler = TaskHandler::new(api_client, storage, exporter, handle, msg_sender)?;
    info!("TaskHandler initialized");

    let core = Core::new(task_handler.clone())?;
    info!("Core initialized");

    let status_manager = StatusManager::new();

    info!("Setting up Tauri application");

    tauri::async_runtime::spawn(msg_loop(
        app.handle().clone(),
        core.task_manager().clone(),
        msg_receiver,
    ));
    app.manage(core);
    app.manage(task_handler);
    app.manage(Mutex::new(sdk_api_client.clone()));

    if let Ok(session) = Session::load(main_config.session_path.as_path()) {
        tauri::async_runtime::spawn_blocking(async move || {
            if let Err(e) = sdk_api_client.login_with_session(session).await {
                error!("{e}");
            }
        });
    }
    app.manage(status_manager);

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
