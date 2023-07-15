#![allow(unused)]
use bytes::Bytes;
use std::fs;
use std::path::Path;
use anyhow::Result;
use serde_json::{from_str, to_string, to_value, Value};

const LOGIN_INFO_PATH_STR: &str = "res/login_info.json";
pub type LoginInfo = Value;

#[derive(Clone)]
pub enum LoginState {
    GettingQRCode,
    QRCodeGotten(egui::ImageData),
    Logged(Config),
}

impl Default for LoginState {
    fn default() -> Self {
        Self::GettingQRCode
    }
}

pub struct Loginator;

impl Loginator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_login_qrcode(&self) -> Result<egui::ImageData> {
        todo!()
    }

    pub async fn check(&self) -> Result<bool> {
        todo!()
    }

    pub async fn get_cookie(&self) -> Result<String> {
        todo!()
    }

    pub async fn get_uid(&self) -> Result<String> {
        todo!()
    }

    pub async fn get_mobile_cookie(&self) -> Result<String> {
        todo!()
    }
}

impl Default for Loginator {
    fn default() -> Self {
        Self {}
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
