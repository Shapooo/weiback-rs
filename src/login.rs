use bytes::Bytes;

use crate::error::Result;

pub struct Loginator;

impl Loginator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_login_qrcode(&self) -> Result<Bytes> {
        todo!()
    }

    pub async fn check(&self) -> Result<()> {
        todo!()
    }

    pub async fn get_cookie(&self) -> Result<String> {
        todo!()
    }

    pub async fn get_mobile_cookei(&self) -> Result<String> {
        todo!()
    }
}
