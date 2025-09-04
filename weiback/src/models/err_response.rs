use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ErrResponse {
    pub errmsg: String,
    pub errno: i32,
    pub errtype: String,
    pub isblock: bool,
}

impl From<weibosdk_rs::api_client::ErrResponse> for ErrResponse {
    fn from(value: weibosdk_rs::api_client::ErrResponse) -> Self {
        Self {
            errmsg: value.errmsg,
            errno: value.errno,
            errtype: value.errtype,
            isblock: value.isblock,
        }
    }
}
