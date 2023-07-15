use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use chrono;
use egui::ColorImage;
use egui::ImageData;
use image::io::Reader;
use reqwest::{
    blocking::Client,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};
use reqwest_cookie_store::CookieStoreMutex;
use serde_json::{from_str, to_string, to_value, Value};

const LOGIN_INFO_PATH_STR: &str = "res/login_info.json";
const LOGIN_API: &str =
    "https://login.sina.com.cn/sso/qrcode/image?entry=weibo&size=109&callback=STK_";

pub type LoginInfo = Value;

#[derive(Clone)]
pub enum LoginState {
    GettingQRCode,
    QRCodeGotten(egui::ImageData),
    Confirmed,
    Logged(LoginInfo),
}

impl Default for LoginState {
    fn default() -> Self {
        Self::GettingQRCode
    }
}

pub struct Loginator {
    cookie_store: Arc<CookieStoreMutex>,
    client: Client,
}

impl Loginator {
    pub fn new() -> Self {
        let headers = HeaderMap::from_iter([
            (
                header::USER_AGENT,
                HeaderValue::from_static(
                    "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/115.0",
                ),
            ),
            (header::ACCEPT, HeaderValue::from_static("*/*")),
            (
                header::ACCEPT_LANGUAGE,
                HeaderValue::from_static("en-US,en;q=0.5"),
            ),
            (
                header::ACCEPT_ENCODING,
                HeaderValue::from_static("gzip, deflate, br"),
            ),
            (header::DNT, HeaderValue::from_static("1")),
            (header::CONNECTION, HeaderValue::from_static("keep-alive")),
            (
                header::REFERER,
                HeaderValue::from_static("https://weibo.com"),
            ),
            (header::TE, HeaderValue::from_static("trailers")),
            (
                HeaderName::from_static("sec-fetch-mode"),
                HeaderValue::from_static("no-cors"),
            ),
            (
                HeaderName::from_static("sec-fetch-site"),
                HeaderValue::from_static("cross-site"),
            ),
        ]);

        let cookie_store = Arc::new(CookieStoreMutex::default());
        let client = Client::builder()
            .cookie_store(true)
            .cookie_provider(cookie_store.clone())
            .default_headers(headers)
            .build()
            .unwrap();
        Self {
            client,
            cookie_store,
        }
    }

    pub fn get_login_qrcode(&self) -> Result<egui::ImageData> {
    }

    pub fn wait_confirm(&self) -> Result<()> {
        todo!()
    }

    pub fn wait_login(self) -> Result<LoginInfo> {
        todo!()
    }

    fn login_weibo_com(&self) -> Result<()> {
        todo!()
    }

    fn login_m_weibo_cn(&self) -> Result<()> {
        todo!()
    }
}

impl Default for Loginator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_login_info() -> Result<Option<LoginInfo>> {
    let path = Path::new(LOGIN_INFO_PATH_STR);
    if !path.exists() {
        return Ok(None);
    } else if !path.is_file() {
        return Err(anyhow::anyhow!("login info path have been occupied"));
    }
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
