use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use egui::ColorImage;
use image::ImageReader;
use log::debug;
use reqwest::{
    blocking::Client,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde_json::{from_str, from_value, to_string, to_value, Value};

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
    index: i64,
    uid: i64,
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
            uid: Default::default(),
            index: 3,
        }
    }

    pub fn get_cookie_store(&self) -> Arc<CookieStoreMutex> {
        self.cookie_store.clone()
    }

    pub fn get_login_qrcode(&mut self) -> Result<egui::ImageData> {
        self.time_stamp = chrono::Local::now().timestamp_millis();
        let text = self
            .client
            .get(format!("{}{}1", LOGIN_API, self.time_stamp))
            .send()?
            .text()?;
        let text = get_text(&text).ok_or(anyhow::anyhow!(
            "cannot find wanted json in returned text: {}",
            text
        ))?;
        let json: Value = from_str(text)?;
        let img_url = String::from("http:")
            + json["data"]["image"]
                .as_str()
                .expect("qrcode image url get failed");
        self.qrid = json["data"]["qrid"]
            .as_str()
            .expect("qrcode id get failed")
            .into();
        let img = self.client.get(img_url).send()?.bytes()?;
        let img = ImageReader::new(Cursor::new(img))
            .with_guessed_format()?
            .decode()?
            .into_rgb8();
        let img = ColorImage::from_rgb(
            [img.width() as usize, img.height() as usize],
            &img.into_vec()[..],
        );
        Ok(egui::ImageData::Color(img.into()))
    }

    pub fn wait_confirm(&mut self) -> Result<()> {
        loop {
            let url = format!(
                "https://login.sina.com.cn/sso/qrcode/check?entry=weibo&qrid={}&callback=STK_{}{}",
                self.qrid, self.time_stamp, self.index
            );
            let text = self.client.get(url).send()?.text()?;
            let text = get_text(&text).ok_or(anyhow::anyhow!(
                "cannot find wanted json in returned text: {}",
                text
            ))?;
            let mut json: Value = from_str(text)?;
            debug!("login check ret: {:?}", json);
            let retcode = json["retcode"]
                .as_i64()
                .ok_or(anyhow::anyhow!("retcode of check api should be a number"))?;
            match retcode {
                20000000 => {
                    self.alt = if let Value::String(alt) = json["data"]["alt"].take() {
                        alt
                    } else {
                        return Err(anyhow::anyhow!(
                            "no alt field in login confirm json: {}",
                            text
                        ));
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
            self.index += 2;
            sleep(Duration::from_secs(2));
        }
    }

    pub fn wait_login(mut self) -> Result<LoginInfo> {
        self.login_weibo_com()?;
        self.login_m_weibo_cn()?;
        drop(self.client);
        let cookie_store = Arc::try_unwrap(self.cookie_store)
            .or(Err(anyhow::anyhow!(
                "unwrap Arc<CookieStoreMutext> failed, there are bugs"
            )))?
            .into_inner()?;
        let login_info = to_login_info(self.uid, &cookie_store)?;
        Ok(login_info)
    }

    fn login_weibo_com(&mut self) -> Result<()> {
        let url = format!(
            "https://login.sina.com.cn/sso/login.php?entry=weibo&returntype=TEXT\
            &crossdomain=1&cdult=3&domain=weibo&alt={}&savestate=30&callback=STK_{}{}",
            self.alt, self.time_stamp, self.index
        );
        self.index += 2;
        let text = self.client.get(url).send()?.text()?;
        let text = get_text(&text).ok_or(anyhow::anyhow!(
            "cannot find wanted json in returned text: {}",
            text
        ))?;
        let mut json: Value = from_str(text)?;
        self.uid = if let Value::String(uid) = json["uid"].take() {
            uid.parse()?
        } else {
            return Err(anyhow::anyhow!("no uid field in returned json"));
        };
        if let Value::Array(url_list) = json["crossDomainUrlList"].take() {
            for url in url_list {
                self.client
                    .get(url.as_str().ok_or(anyhow::anyhow!(
                        "crossDomainUrlList must be a vec of string in login weibo.com returned"
                    ))?)
                    .send()?;
            }
        } else {
            return Err(anyhow::anyhow!(
                "crossDomainUrlList should be a list of url"
            ));
        }
        self.client.get("https://weibo.com/login.php").send()?;
        Ok(())
    }

    fn login_m_weibo_cn(&mut self) -> Result<()> {
        let mobile_ua = HeaderValue::from_static(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) \
             AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 \
             Mobile/15E148 Safari/604.1 Edg/116.0.0.0",
        );
        self.client
            .get("https://m.weibo.cn")
            .header(header::USER_AGENT, mobile_ua.clone())
            .send()?;

        Ok(())
    }
}

fn get_text(text: &str) -> Option<&str> {
    let len = text.len();
    text.find('(').map(|start| &text[start + 1..len - 2])
}

impl Default for Loginator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_login_info() -> Result<Option<LoginInfo>> {
    let path = std::env::current_exe()?
        .parent()
        .ok_or(anyhow::anyhow!(
            "the executable: {:?} should have parent, maybe bugs in there",
            std::env::current_exe()
        ))?
        .join(LOGIN_INFO_PATH_STR);
    if !path.exists() {
        return Ok(None);
    } else if !path.is_file() {
        return Err(anyhow::anyhow!("login info path have been occupied"));
    }
    debug!("login_info.json exists, start to validate");
    let content = fs::read_to_string(path)?;
    let login_info: Value = from_str(&content)?;
    let cookie_store: CookieStore = from_value(login_info["cookies"].clone())?;
    let headers = HeaderMap::from_iter([
        (
            header::ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        ),
        (
            header::REFERER,
            HeaderValue::from_static("https://weibo.com/"),
        ),
        (
            header::USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) \
                     Gecko/20100101 Firefox/113.0",
            ),
        ),
        (
            header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.5"),
        ),
        (
            header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br"),
        ),
        (
            HeaderName::from_static("x-requested-with"),
            HeaderValue::from_static("XMLHttpRequest"),
        ),
        (
            HeaderName::from_static("client-version"),
            HeaderValue::from_static("v2.40.55"),
        ),
        (
            HeaderName::from_static("server-version"),
            HeaderValue::from_static("v2023.05.23.3"),
        ),
        (header::DNT, HeaderValue::from_static("1")),
        (
            HeaderName::from_static("sec-fetch-dest"),
            HeaderValue::from_static("empty"),
        ),
        (
            HeaderName::from_static("sec-fetch-mode"),
            HeaderValue::from_static("cors"),
        ),
        (
            HeaderName::from_static("sec-fetch-site"),
            HeaderValue::from_static("same-origin"),
        ),
        (header::TE, HeaderValue::from_static("trailers")),
    ]);
    let temporary_client = Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .cookie_provider(Arc::new(CookieStoreMutex::new(cookie_store)))
        .build()?;
    let res: Value = temporary_client
        .get("https://weibo.com/ajax/favorites/tags?page=1&is_show_total=1")
        .send()?
        .json()?;
    debug!("login check return: {:?}", res);
    if res["ok"] == 1 {
        debug!("cookie is valid");
        Ok(Some(login_info))
    } else {
        debug!("cookie is invalid");
        Ok(None)
    }
}

pub fn to_login_info(uid: i64, cookie_store: &CookieStore) -> Result<LoginInfo> {
    let cookie_json = cookie_store.iter_any().collect::<Vec<_>>();
    let login_info = serde_json::json!({"uid":uid, "cookies":cookie_json});
    Ok(to_value(login_info)?)
}

pub fn save_login_info(login_info: &LoginInfo) -> Result<()> {
    let login_info_file = std::env::current_exe()?
        .parent()
        .ok_or(anyhow::anyhow!(
            "the executable: {:?} should have parent, maybe bugs in there",
            std::env::current_exe()
        ))?
        .join(LOGIN_INFO_PATH_STR);
    fs::write(login_info_file, to_string(login_info)?)?;
    Ok(())
}
