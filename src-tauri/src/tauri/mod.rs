use std::ops::RangeInclusive;
use std::sync::Arc;

use tauri::{self, Manager, State};
use tokio::sync::{Mutex, mpsc};
use weibosdk_rs::{WeiboAPIImpl as WAI, client::new_client_with_headers, weibo_api::LoginState};

use crate::config::get_config;
use crate::core::{BFOptions, BUOptions, Core, TaskRequest, task_handler::TaskHandler};
use crate::error::{Error, Result};
use crate::exporter::{ExportOptions, ExporterImpl};
use crate::media_downloader::MediaDownloaderImpl;
use crate::storage::StorageImpl;

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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let core = Core::new(app.handle().clone(), task_handler.clone(), msg_receiver)?;
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
