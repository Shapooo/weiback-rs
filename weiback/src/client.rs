use std::path::PathBuf;

use reqwest::Response as RqstResponse;
use tokio::fs::write;
use weibosdk_rs::client::{Client as SDKClient, CookieStore, HttpClient, HttpResponse};

#[derive(Debug, Clone)]
pub struct Client {
    client: SDKClient,
    dev_mode_out_dir: Option<PathBuf>,
}

impl Client {
    pub fn new(client: SDKClient, dev_mode_out_dir: Option<PathBuf>) -> Self {
        Self {
            client,
            dev_mode_out_dir,
        }
    }

    pub fn main_client(&self) -> &reqwest::Client {
        self.client.main_client()
    }

    pub fn web_client(&self) -> Option<&reqwest::Client> {
        self.client.web_client()
    }
}

impl HttpClient for Client {
    type Response = Response;
    async fn get(
        &self,
        url: &str,
        query: &(impl serde::Serialize + Send + Sync),
        retry_times: u8,
    ) -> weibosdk_rs::Result<Self::Response> {
        let res = self.client.get(url, query, retry_times).await?;
        Ok(Response::new(res, self.dev_mode_out_dir.clone()))
    }

    async fn post(
        &self,
        url: &str,
        form: &(impl serde::Serialize + Send + Sync),
        retry_times: u8,
    ) -> weibosdk_rs::Result<Self::Response> {
        let res = self.client.post(url, form, retry_times).await?;
        Ok(Response::new(res, self.dev_mode_out_dir.clone()))
    }

    fn set_cookie(&mut self, cookie_store: CookieStore) -> weibosdk_rs::Result<()> {
        self.client.set_cookie(cookie_store)
    }
}

pub struct Response {
    pub res: RqstResponse,
    pub output_dir: Option<PathBuf>,
}

impl Response {
    pub fn new(res: RqstResponse, output_dir: Option<PathBuf>) -> Self {
        Self { res, output_dir }
    }
}

impl HttpResponse for Response {
    async fn json<T: serde::de::DeserializeOwned>(self) -> weibosdk_rs::Result<T> {
        if let Some(path) = self.output_dir {
            let txt = self.res.text().await?;
            let path = path.join("tempname"); // TODO: use uuid + time file name
            write(path, &txt).await?;
            Ok(serde_json::from_str::<T>(&txt)?)
        } else {
            Ok(self.res.json::<T>().await?)
        }
    }

    async fn text(self) -> weibosdk_rs::Result<String> {
        if let Some(path) = self.output_dir {
            let txt = self.res.text().await?;
            let path = path.join("tempname"); // TODO: use uuid + time file name
            write(path, &txt).await?;
            Ok(txt)
        } else {
            Ok(self.res.text().await?)
        }
    }

    async fn bytes(self) -> weibosdk_rs::Result<bytes::Bytes> {
        if let Some(path) = self.output_dir {
            let bt = self.res.bytes().await?;
            let path = path.join("tempname"); // TODO: use uuid + time file name
            write(path, &bt).await?;
            Ok(bt)
        } else {
            Ok(self.res.bytes().await?)
        }
    }
}
