mod error;

use std::ops::RangeInclusive;

use log::{debug, error, info, warn};
use tauri::{self, App, AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;

use weiback::api::ApiClientImpl;
use weiback::config::get_config;
use weiback::core::{
    BFOptions, BUOptions, Core, ExportOptions, TaskRequest, task_handler::TaskHandler,
    task_manager::TaskManger,
};
#[cfg(feature = "dev-mode")]
use weiback::dev_client::DevClient;
use weiback::exporter::ExporterImpl;
use weiback::media_downloader::{MediaDownloaderHandle, create_downloader};
use weiback::message::{ErrMsg, Message, TaskProgress};
use weiback::storage::StorageImpl;
use weibosdk_rs::{
    ApiClient as SdkApiClient, Client as HttpClient, api_client::LoginState, session::Session,
};

use error::{Error, Result};

#[cfg(feature = "dev-mode")]
type TH = TaskHandler<weiback::api::DevApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;
#[cfg(not(feature = "dev-mode"))]
type TH =
    TaskHandler<weiback::api::DefaultApiClient, StorageImpl, ExporterImpl, MediaDownloaderHandle>;
#[cfg(feature = "dev-mode")]
type CurrentSdkApiClient = SdkApiClient<DevClient>;
#[cfg(not(feature = "dev-mode"))]
type CurrentSdkApiClient = SdkApiClient<HttpClient>;

#[tauri::command]
async fn backup_self(
    core: State<'_, Core>,
    api_client: State<'_, SdkApiClient<HttpClient>>,
    range: RangeInclusive<u32>,
) -> Result<()> {
    info!("backup_self called with range: {range:?}");
    let uid = api_client.session()?.uid.clone();
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
async fn send_code(api_client: State<'_, CurrentSdkApiClient>, phone_number: String) -> Result<()> {
    info!(
        "send_code called for phone number (partially hidden): ...{}",
        &phone_number.chars().skip(7).collect::<String>()
    );
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
async fn login(api_client: State<'_, SdkApiClient<HttpClient>>, sms_code: String) -> Result<()> {
    info!("login called with a sms_code");
    match api_client.login_state() {
        LoginState::WaitingForCode { .. } => {
            info!("Attempting to login with SMS code.");
            api_client.login(&sms_code).await?;
            info!("Login successful.");
            let session_path = get_config()
                .read()
                .expect("config lock failed")
                .session_path
                .clone();
            api_client.session()?.save(session_path)?;
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
fn login_status(api_client: State<'_, CurrentSdkApiClient>) -> Result<Option<String>> {
    info!("get login status");
    Ok(api_client.session().ok().map(|s| s.uid.to_owned()))
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
    let dev_client = DevClient::new(http_client.clone(), main_config.dev_mode_out_dir.clone());
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

    info!("Setting up Tauri application");
    tauri::async_runtime::spawn(msg_loop(
        app.handle().clone(),
        core.task_manager().clone(),
        msg_receiver,
    ));
    app.manage(core);
    app.manage(task_handler);
    app.manage(sdk_api_client.clone());

    let session_path = main_config.session_path.clone();
    if let Ok(session) = Session::load(session_path.as_path()) {
        tauri::async_runtime::spawn(async move {
            if let Err(e) = sdk_api_client.login_with_session(session).await {
                error!("login with session failed: {e}");
            }
            let Ok(session) = sdk_api_client
                .session()
                .map_err(|e| error!("get new session failed: {e}"))
            else {
                return;
            };
            if let Err(e) = session.save(session_path) {
                error!("save new session failed: {e}");
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
