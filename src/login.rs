use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use chrono;
use egui::ColorImage;
use image::io::Reader;
use log::debug;
use reqwest::{
    blocking::Client,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};
use reqwest_cookie_store::CookieStoreMutex;
use serde_json::{from_str, to_string, to_value, Value};

const LOGIN_INFO_PATH_STR: &str = "res/login_info.json";
const LOGIN_API: &str =
    "https://login.sina.com.cn/sso/qrcode/image?entry=weibo&size=180&callback=STK_";

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
    time_stamp: i64,
    qrid: String,
    alt: String,
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
            time_stamp: 0,
            qrid: Default::default(),
            alt: Default::default(),
        }
    }

    pub fn get_login_qrcode(&mut self) -> Result<egui::ImageData> {
        self.time_stamp = chrono::Local::now().timestamp_millis();
        let text = self
            .client
            .get(format!("{}{}1", LOGIN_API, self.time_stamp))
            .send()?
            .text()?;
        dbg!(&text);
        let text = get_text(&text);
        let mut json: Value = from_str(text)?;
        dbg!(&json);
        let img_url = String::from("http:") + json["data"]["image"].as_str().unwrap();
        self.qrid = if let Value::String(id) = json["data"]["qrid"].take() {
            id
        } else {
            "".into()
        };
        let img = self.client.get(img_url).send()?.bytes()?;
        let img = Reader::new(Cursor::new(img))
            .with_guessed_format()?
            .decode()?
            .into_rgb8();
        let img = ColorImage::from_rgb(
            [img.width() as usize, img.height() as usize],
            &img.into_vec()[..],
        );
        Ok(egui::ImageData::Color(img))
    }

    pub fn wait_confirm(&mut self) -> Result<()> {
        let mut index = 3;
        loop {
            let url = format!(
                "https://login.sina.com.cn/sso/qrcode/check?entry=weibo&qrid={}&callback=STK_{}{}",
                self.qrid, self.time_stamp, index
            );
            let text = self.client.get(url).send()?.text()?;
            let text = get_text(&text);
            let mut json: Value = from_str(text)?;
            debug!("login check ret: {:?}", json);
            let retcode = json["retcode"].as_i64().unwrap();
            match retcode {
                20000000 => {
                    self.alt = if let Value::String(alt) = json["data"]["alt"].take() {
                        alt
                    } else {
                        "".into()
                    };
                    return Ok(());
                }
                50114001 | 50114002 => {}
                _ => {
                    return Err(anyhow::anyhow!(
                        "unexpected retcode, maybe something get wrong"
                    ))
                }
            }
            index += 2;
            sleep(Duration::from_secs(2));
        }
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

fn get_text(text: &str) -> &str {
    let len = text.len();
    &text[text.find('(').unwrap() + 1..len - 2]
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
