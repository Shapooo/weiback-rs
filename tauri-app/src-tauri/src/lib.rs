use std::ops::RangeInclusive;
use std::sync::Arc;

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
    core: State<'_, Core>,
    api_client: State<'_, WeiboAPIImpl>,
    range: RangeInclusive<u32>,
) -> Result<()> {
    let uid = api_client.session()?.uid.clone();
    backup_user(core, uid, range).await
}

#[tauri::command]
async fn backup_user(core: State<'_, Core>, uid: String, range: RangeInclusive<u32>) -> Result<()> {
    core.backup_user(TaskRequest::BackupUser(BUOptions {
        uid: uid.parse().map_err(|err| Error::Other(format!("{err}")))?,
        range,
    }))
    .await
}

#[tauri::command]
async fn backup_favorites(core: State<'_, Core>, range: RangeInclusive<u32>) -> Result<()> {
    core.backup_favorites(TaskRequest::BackupFavorites(BFOptions { range }))
        .await
}

#[tauri::command]
async fn unfavorite_posts(core: State<'_, Core>) -> Result<()> {
    core.unfavorite_posts().await
}

#[tauri::command]
async fn export_from_local(task_handler: State<'_, TH>, range: RangeInclusive<u32>) -> Result<()> {
    let options = ExportOptions::new().range(range);
    task_handler.export_from_local(options).await
}

#[tauri::command]
async fn send_code(api_client: State<'_, Mutex<WeiboAPIImpl>>, phone_number: String) -> Result<()> {
    let mut api_client = api_client.lock().await;
    match api_client.login_state() {
        LoginState::LoggedIn { .. } => Ok(()),
        _ => {
            api_client.get_sms_code(phone_number).await?;
            Ok(())
        }
    }
}

#[tauri::command]
async fn login(api_client: State<'_, Mutex<WeiboAPIImpl>>, sms_code: String) -> Result<()> {
    let mut api_client = api_client.lock().await;
    match api_client.login_state() {
        LoginState::WaitingForCode { .. } => {
            api_client.login(&sms_code).await?;
            Ok(())
        }
        LoginState::LoggedIn { .. } => Ok(()),
        LoginState::Init => Err(Error::Other(
            "FATAL: wrong login state to login".to_string(),
        )),
    }
}

pub fn run() -> Result<()> {
    let weibo_api_config = get_config()
        .read()
        .map_err(|e| Error::Other(e.to_string()))?
        .weibo_api_config
        .clone();
    let (msg_sender, msg_receiver) = mpsc::channel(100);
    let storage = StorageImpl::new().unwrap();
    let storage = Arc::new(storage);
    let exporter = ExporterImpl::new();
    let http_client = new_client_with_headers().unwrap();
    let downloader = MediaDownloaderImpl::new(http_client.clone(), msg_sender.clone());
    let api_client = WeiboAPIImpl::new(http_client.clone(), weibo_api_config);
    let task_handler = TaskHandler::new(
        api_client.clone(),
        storage,
        exporter,
        downloader,
        msg_sender,
    )
    .unwrap();
    let core = Core::new(task_handler.clone())?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            tauri::async_runtime::spawn(msg_loop(
                app.handle().clone(),
                core.task_manager().clone(),
                msg_receiver,
            ));
            app.manage(Mutex::new(core));
            app.manage(task_handler);
            app.manage(Mutex::new(api_client));
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
    loop {
        tokio::select! {
            Some(msg) = msg_receiver.recv() => {
                let _ = handle_task_responses(&app, &task_manager, msg).await; // TODO
            }
            else => {
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
        }) => app.emit(
            "error",
            serde_json::json!({
                "type": r#type,
                "task_id":task_id,
                "err": err.to_string(),
            }),
        )?,
    }
    Ok(())
}
