use std::fmt::Debug;

use tauri::ipc::InvokeError;
use thiserror::Error;
use tokio::sync::{mpsc::error::SendError, oneshot::error::RecvError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}: {1}")]
    Context(String, Box<Error>),

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

    #[error("Url parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Tokio task-related error: {0}")]
    Tokio(String),

    #[error("DateTime parsing error: {0}")]
    DateTime(#[from] chrono::ParseError),

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

impl From<RecvError> for Error {
    fn from(e: RecvError) -> Self {
        Error::Tokio(e.to_string())
    }
}

pub trait Context<T, E> {
    fn context(self, context: &'static str) -> Result<T>;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn context(self, context: &'static str) -> Result<T> {
        self.map_err(|e| Error::Context(context.to_string(), Box::new(e.into())))
    }
}

impl Into<InvokeError> for Error {
    fn into(self) -> InvokeError {
        todo!()
    }
}
