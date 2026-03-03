//! This module defines the standard error response structure received from the Weibo API.
//!
//! It is used to encapsulate API-specific error messages, error codes, and other
//! related information. It also provides a conversion from the SDK's `ErrResponse`
//! to the application's internal `ErrResponse`.
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ErrResponse {
    /// The error message returned by the API.
    pub errmsg: String,
    /// The numeric error code.
    pub errno: i32,
    /// The type of error.
    pub errtype: String,
    /// Indicates if the request was blocked.
    pub isblock: bool,
}

impl From<weibosdk_rs::api_client::ErrResponse> for ErrResponse {
    /// Converts an `ErrResponse` from the `weibosdk_rs` crate into the application's `ErrResponse`.
    ///
    /// # Arguments
    /// * `value` - The `weibosdk_rs::api_client::ErrResponse` to convert.
    ///
    /// # Returns
    /// An `ErrResponse` model.
    fn from(value: weibosdk_rs::api_client::ErrResponse) -> Self {
        Self {
            errmsg: value.errmsg,
            errno: value.errno,
            errtype: value.errtype,
            isblock: value.isblock,
        }
    }
}
