#![allow(unused)]
use bytes::Bytes;

use crate::config::Config;
use crate::error::Result;

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
