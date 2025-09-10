use std::{
    fs::read_to_string,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use tokio::fs::write;
use uuid::Uuid;
use weibosdk_rs::{
    Client,
    http_client::{HttpClient, HttpResponse},
};

const RECORDS_FN: &str = "records.json";

#[derive(Debug, Clone)]
pub struct DevClient {
    client: Client,
    dev_mode_out_dir: Option<PathBuf>,
}

impl DevClient {
    pub fn new(client: Client, dev_mode_out_dir: Option<PathBuf>) -> Self {
        if let Some(path) = dev_mode_out_dir.clone() {
            let records_s =
                read_to_string(path.join(RECORDS_FN)).expect("read records.json failed");
            let records = from_str(&records_s).expect("parse records failed");
            RECORDS
                .set(Record {
                    records: Arc::new(Mutex::new(records)),
                    path,
                })
                .expect("set RECORDS failed");
        }
        Self {
            client,
            dev_mode_out_dir,
        }
    }
}

impl HttpClient for DevClient {
    type Response = DevResponse;

    async fn get(
        &self,
        url: &str,
        query: &(impl serde::Serialize + Send + Sync),
        retry_times: u8,
    ) -> weibosdk_rs::error::Result<Self::Response> {
        let res = self.client.get(url, query, retry_times).await?;
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
    ) -> weibosdk_rs::error::Result<Self::Response> {
        let res = self.client.post(url, form, retry_times).await?;
        Ok(DevResponse::new(
            res,
            self.dev_mode_out_dir.clone(),
            url.to_string(),
            to_string(form)
                .map_err(|e| weibosdk_rs::error::Error::DataConversionError(e.to_string()))?,
        ))
    }

    fn set_cookie(
        &mut self,
        cookie_store: weibosdk_rs::http_client::CookieStore,
    ) -> weibosdk_rs::error::Result<()> {
        self.client.set_cookie(cookie_store)
    }
}

#[derive(Debug)]
pub struct DevResponse {
    pub res: Response,
    pub output_dir: Option<PathBuf>,
    pub url: String,
    pub query: String,
}

impl DevResponse {
    pub fn new(res: Response, output_dir: Option<PathBuf>, url: String, query: String) -> Self {
        Self {
            res,
            output_dir,
            url,
            query,
        }
    }
}

impl HttpResponse for DevResponse {
    async fn json<T: serde::de::DeserializeOwned>(self) -> weibosdk_rs::error::Result<T> {
        if let Some(path) = self.output_dir {
            let txt = self.res.text().await?;
            let file_name = Uuid::now_v7().simple().to_string();
            let path = path.join(&file_name);
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

    async fn text(self) -> weibosdk_rs::error::Result<String> {
        if let Some(path) = self.output_dir {
            let txt = self.res.text().await?;
            let file_name = Uuid::now_v7().simple().to_string();
            let path = path.join(&file_name);
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

    async fn bytes(self) -> weibosdk_rs::error::Result<bytes::Bytes> {
        if let Some(path) = self.output_dir {
            let bt = self.res.bytes().await?;
            let file_name = Uuid::now_v7().simple().to_string();
            let path = path.join(&file_name);
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

fn append_record(item: RecordItem) {
    if let Some(records) = RECORDS.get() {
        let mut records = records.records.lock().unwrap();
        records.push(item);
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RecordItem {
    url: String,
    query: String,
    file_name: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct Record {
    pub records: Arc<Mutex<Vec<RecordItem>>>,
    pub path: PathBuf,
}

static RECORDS: OnceLock<Record> = OnceLock::new();

impl Drop for Record {
    fn drop(&mut self) {
        let records = self.records.lock().unwrap();
        let s = to_string::<Vec<_>>(records.as_ref()).unwrap();
        std::fs::write(self.path.join(RECORDS_FN), s).unwrap();
    }
}
