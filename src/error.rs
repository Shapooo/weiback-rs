use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub type Result<T> = anyhow::Result<T>;

#[derive(Debug, Error)]
pub enum TokioError {
    #[error("Failed to send data through channel: {0}")]
    SendError(String),
}

// Manually implement From for SendError<T> because it's generic.
// We convert it to a String to make TokioError non-generic.
impl<T> From<SendError<T>> for TokioError
where
    T: Debug,
{
    fn from(err: SendError<T>) -> Self {
        TokioError::SendError(err.to_string())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Template rendering error: {0}")]
    Tera(#[from] tera::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Weibo api client error: {0}")]
    Client(#[from] weibosdk_rs::Error),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Tokio task-related error: {0}")]
    Tokio(#[from] TokioError),

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("An unexpected error occurred: {0}")]
    Other(String),
}
