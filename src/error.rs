use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub type Result<T> = std::result::Result<T, Error>;

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
    Tokio(String),

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("An unexpected error occurred: {0}")]
    Other(String),
}

impl<T> From<SendError<T>> for Error {
    fn from(e: SendError<T>) -> Self {
        Error::Tokio(e.to_string())
    }
}
