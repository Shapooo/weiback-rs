use std::fmt::Debug;

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
    DbError(String),

    #[error("Template rendering error: {0}")]
    Tera(#[from] tera::Error),

    #[error("Parse/deserialize error: {0}")]
    FormatError(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Api error: {0:?}")]
    ApiError(weibosdk_rs::models::ErrResponse),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("Tokio task-related error: {0}")]
    Tokio(String),

    #[error("Task related error: {0}")]
    InconsistentTask(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Not logged in")]
    NotLoggedIn,
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

impl From<weibosdk_rs::Error> for Error {
    fn from(error: weibosdk_rs::Error) -> Self {
        use weibosdk_rs::Error as SDKError;
        match error {
            SDKError::IoError(e) => Error::Io(e),
            SDKError::NotLoggedIn => Error::NotLoggedIn,
            SDKError::NetworkError(e) => Error::Network(e),
            SDKError::ApiError(e) => Error::ApiError(e),
            SDKError::DataConversionError(e) => Error::FormatError(e),
            SDKError::DeserializationError(e) => Error::FormatError(e.to_string()),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Self::Lock(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::FormatError(err.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Self::FormatError(err.to_string())
    }
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Self::FormatError(err.to_string())
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::DbError(err.to_string())
    }
}
