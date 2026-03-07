//! This module provides a development-focused HTTP client that records API responses.
//!
//! When `dev-mode` is enabled, the [`DevClient`] intercepts HTTP requests and saves
//! the resulting JSON, text, or binary data to a local directory. It also maintains
//! a `records.json` file that maps request URLs and queries to these local files.
//! This is invaluable for creating mock data and debugging without hitting the live API.

use std::{
    fs::{create_dir_all, read_to_string},
    ops::Deref,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use tokio::fs::write;
use tracing::{debug, error, info};
use uuid::Uuid;
use weibosdk_rs::{
    Client,
    http_client::{HttpClient, HttpResponse},
};

const RECORDS_FN: &str = "records.json";

/// An HTTP client that wraps a standard [`Client`] and records responses.
#[derive(Debug, Clone)]
pub struct DevClient {
    client: Client,
    dev_mode_out_dir: Option<PathBuf>,
}

impl DevClient {
    /// Creates a new `DevClient`.
    ///
    /// If `dev_mode_out_dir` is provided, it will attempt to load existing records
    /// from `records.json` in that directory.
    ///
    /// # Arguments
    /// * `client` - The underlying standard HTTP client.
    /// * `dev_mode_out_dir` - The directory where responses and records should be saved.
    pub fn new(client: Client, dev_mode_out_dir: Option<PathBuf>) -> Self {
        if let Some(path) = dev_mode_out_dir.clone() {
            let json_path = path.join(RECORDS_FN);
            if json_path.exists() {
                info!("Loading existing records from {:?}", json_path);
                let records_s =
                    read_to_string(json_path.as_path()).expect("read records.json failed");
                let records = from_str(&records_s).unwrap_or_default();
                let records = Record {
                    records: Arc::new(Mutex::new(records)),
                    path,
                };
                RECORDS.set(records).expect("set RECORDS failed");
            } else {
                info!(
                    "No records file found at {:?}, creating a new one.",
                    json_path
                );
                create_dir_all(json_path.parent().unwrap()).expect("create dev records dir failed");
                let records = Record {
                    records: Default::default(),
                    path,
                };
                RECORDS.set(records).expect("set RECORDS failed");
            }
        }
        Self {
            client,
            dev_mode_out_dir,
        }
    }
}

#[async_trait]
impl HttpClient for DevClient {
    type Response = DevResponse;

    async fn get(
        &self,
        url: &str,
        query: &(impl serde::Serialize + Send + Sync),
        retry_times: u8,
        timeout: Duration,
    ) -> weibosdk_rs::error::Result<Self::Response> {
        let res = self.client.get(url, query, retry_times, timeout).await?;
        Ok(DevResponse::new(
            res,
            self.dev_mode_out_dir.clone(),
            url.to_string(),
            to_string(query)
                .map_err(|e| weibosdk_rs::error::Error::DataConversionError(e.to_string()))?,
        ))
    }

    async fn post(
        &self,
        url: &str,
        form: &(impl serde::Serialize + Send + Sync),
        retry_times: u8,
        timeout: Duration,
    ) -> weibosdk_rs::error::Result<Self::Response> {
        let res = self.client.post(url, form, retry_times, timeout).await?;
        Ok(DevResponse::new(
            res,
            self.dev_mode_out_dir.clone(),
            url.to_string(),
            to_string(form)
                .map_err(|e| weibosdk_rs::error::Error::DataConversionError(e.to_string()))?,
        ))
    }

    fn set_cookie(
        &self,
        cookie_store: weibosdk_rs::http_client::CookieStore,
    ) -> weibosdk_rs::error::Result<()> {
        self.client.set_cookie(cookie_store)
    }
}

/// A wrapper around `reqwest::Response` that records data when consumed.
#[derive(Debug)]
pub struct DevResponse {
    pub res: Response,
    pub output_dir: Option<PathBuf>,
    pub url: String,
    pub query: String,
}

impl DevResponse {
    /// Creates a new `DevResponse`.
    pub fn new(res: Response, output_dir: Option<PathBuf>, url: String, query: String) -> Self {
        Self {
            res,
            output_dir,
            url,
            query,
        }
    }
}

#[async_trait]
impl HttpResponse for DevResponse {
    /// Consumes the response and parses it as JSON, while saving the raw text to disk.
    async fn json<T: serde::de::DeserializeOwned>(self) -> weibosdk_rs::error::Result<T> {
        if let Some(path) = self.output_dir {
            let txt = self.res.text().await?;
            let file_name = Uuid::now_v7().simple().to_string();
            let path = path.join(&file_name);
            info!("Saving response for {} to file {:?}", self.url, &path);
            let record = RecordItem {
                url: self.url,
                query: self.query,
                file_name,
            };
            append_record(record);
            write(path, &txt).await?;
            Ok(serde_json::from_str::<T>(&txt)?)
        } else {
            Ok(self.res.json::<T>().await?)
        }
    }

    /// Consumes the response as text and saves it to disk.
    async fn text(self) -> weibosdk_rs::error::Result<String> {
        if let Some(path) = self.output_dir {
            let txt = self.res.text().await?;
            let file_name = Uuid::now_v7().simple().to_string();
            let path = path.join(&file_name);
            info!("Saving response for {} to file {:?}", self.url, &path);
            let record = RecordItem {
                url: self.url,
                query: self.query,
                file_name,
            };
            append_record(record);
            write(path, &txt).await?;
            Ok(txt)
        } else {
            Ok(self.res.text().await?)
        }
    }

    /// Consumes the response as bytes and saves them to disk.
    async fn bytes(self) -> weibosdk_rs::error::Result<bytes::Bytes> {
        if let Some(path) = self.output_dir {
            let bt = self.res.bytes().await?;
            let file_name = Uuid::now_v7().simple().to_string();
            let path = path.join(&file_name);
            info!("Saving response for {} to file {:?}", self.url, &path);
            let record = RecordItem {
                url: self.url,
                query: self.query,
                file_name,
            };
            append_record(record);
            write(path, &bt).await?;
            Ok(bt)
        } else {
            Ok(self.res.bytes().await?)
        }
    }
}

/// Internal function to add a record item to the global list.
fn append_record(item: RecordItem) {
    if let Some(records) = RECORDS.get() {
        debug!("Appending record for url: {}", &item.url);
        let mut records = records.records.lock().unwrap();
        records.push(item);
    }
}

/// Represents a mapping between a request and its saved response file.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RecordItem {
    url: String,
    query: String,
    file_name: String,
}

/// Internal structure to hold the global record state.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Record {
    pub records: Arc<Mutex<Vec<RecordItem>>>,
    pub path: PathBuf,
}

static RECORDS: OnceLock<Record> = OnceLock::new();

/// Persists all captured records to the `records.json` file.
///
/// This should be called before the application exits to ensure all intercepted
/// requests are indexed correctly.
pub fn save_records() {
    let Some(records) = RECORDS.get() else {
        return;
    };

    let records_path = records.path.join(RECORDS_FN);
    info!("Writing records to {:?}", &records_path);
    let Ok(records) = records.records.lock() else {
        error!("records lock failed");
        return;
    };
    let s = to_string::<Vec<_>>(records.deref()).unwrap();
    std::fs::write(records_path, s).unwrap();
}
