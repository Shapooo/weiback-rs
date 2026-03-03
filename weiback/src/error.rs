//! Defines the custom error types and a convenient `Result` alias for the `weiback` application.
//!
//! This module centralizes error handling, converting various external library errors
//! into a unified `Error` enum. It also provides a `Context` trait for adding
//! contextual information to errors.
use std::fmt::Debug;

use thiserror::Error;
use tokio::sync::{mpsc::error::SendError, oneshot::error::RecvError};

/// A convenient type alias for `std::result::Result` with the custom `Error` type.
pub type Result<T> = std::result::Result<T, Error>;

/// The main error enum for the `weiback` application.
///
/// It encapsulates various types of errors that can occur during application
/// execution, such as I/O errors, database issues, network problems, and
/// data formatting errors.
#[derive(Debug, Error)]
pub enum Error {
    /// An error with additional contextual information.
    #[error("{0}: {1}")]
    Context(String, Box<Error>),

    /// An error originating from standard I/O operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An error related to database operations.
    #[error("Database error: {0}")]
    DbError(String),

    /// An error encountered during template rendering (e.g., using Tera).
    #[error("Template rendering error: {0}")]
    Tera(#[from] tera::Error),

    /// An error during data parsing, serialization, or deserialization.
    #[error("Parse/deserialize error: {0}")]
    FormatError(String),

    /// A network-related error, typically from `reqwest`.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// An error returned by the Weibo API.
    #[error("Api error: {0:?}")]
    ApiError(crate::models::ErrResponse),

    /// An error related to synchronization primitives (e.g., mutex poisoning).
    #[error("Lock error: {0}")]
    Lock(String),

    /// A general error related to Tokio asynchronous runtime operations.
    #[error("Tokio task-related error: {0}")]
    Tokio(String),

    /// An error indicating an inconsistent state within a background task.
    #[error("Task related error: {0}")]
    InconsistentTask(String),

    /// An error related to application configuration.
    #[error("Config error: {0}")]
    ConfigError(String),

    /// Indicates that the user is not logged in to Weibo.
    #[error("Not logged in")]
    NotLoggedIn,
}

impl<T> From<SendError<T>> for Error {
    /// Converts a `tokio::sync::mpsc::error::SendError` into the application's `Error::Tokio` variant.
    ///
    /// # Arguments
    /// * `e` - The `SendError` to convert.
    ///
    /// # Returns
    /// An `Error::Tokio` variant.
    fn from(e: SendError<T>) -> Self {
        Error::Tokio(e.to_string())
    }
}

impl From<RecvError> for Error {
    /// Converts a `tokio::sync::oneshot::error::RecvError` into the application's `Error::Tokio` variant.
    ///
    /// # Arguments
    /// * `e` - The `RecvError` to convert.
    ///
    /// # Returns
    /// An `Error::Tokio` variant.
    fn from(e: RecvError) -> Self {
        Error::Tokio(e.to_string())
    }
}

/// A trait to extend `Result` with a `context` method, allowing for adding
/// descriptive strings to errors.
pub trait Context<T, E> {
    /// Adds a static string context to an error.
    ///
    /// If the `Result` is `Err`, it converts the error into `Error::Context`
    /// with the provided string.
    ///
    /// # Arguments
    /// * `context` - A static string describing the context of the error.
    ///
    /// # Returns
    /// A `Result` with the original `Ok` value or an `Error::Context`.
    fn context(self, context: &'static str) -> Result<T>;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    /// Implements the `Context` trait for standard `Result` types.
    ///
    /// # Arguments
    /// * `context` - A static string describing the context of the error.
    ///
    /// # Returns
    /// A `Result` with the original `Ok` value or an `Error::Context`.
    fn context(self, context: &'static str) -> Result<T> {
        self.map_err(|e| Error::Context(context.to_string(), Box::new(e.into())))
    }
}

impl From<weibosdk_rs::Error> for Error {
    /// Converts errors from `weibosdk_rs` into the application's `Error` enum.
    ///
    /// This maps specific SDK errors to their corresponding `weiback::Error` variants.
    ///
    /// # Arguments
    /// * `error` - The `weibosdk_rs::Error` to convert.
    ///
    /// # Returns
    /// The corresponding `weiback::Error` variant.
    fn from(error: weibosdk_rs::Error) -> Self {
        use weibosdk_rs::Error as SDKError;
        match error {
            SDKError::IoError(e) => Error::Io(e),
            SDKError::NotLoggedIn => Error::NotLoggedIn,
            SDKError::NetworkError(e) => Error::Network(e),
            SDKError::ApiError(e) => Error::ApiError(e.into()),
            SDKError::DataConversionError(e) => Error::FormatError(e),
            SDKError::DeserializationError(e) => Error::FormatError(e.to_string()),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    /// Converts a `std::sync::PoisonError` into the application's `Error::Lock` variant.
    ///
    /// # Arguments
    /// * `err` - The `PoisonError` to convert.
    ///
    /// # Returns
    /// An `Error::Lock` variant.
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Self::Lock(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    /// Converts a `serde_json::Error` into the application's `Error::FormatError` variant.
    ///
    /// # Arguments
    /// * `err` - The `serde_json::Error` to convert.
    ///
    /// # Returns
    /// An `Error::FormatError` variant.
    fn from(err: serde_json::Error) -> Self {
        Self::FormatError(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    /// Converts a `toml::de::Error` into the application's `Error::FormatError` variant.
    ///
    /// # Arguments
    /// * `value` - The `toml::de::Error` to convert.
    ///
    /// # Returns
    /// An `Error::FormatError` variant.
    fn from(value: toml::de::Error) -> Self {
        Self::FormatError(value.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    /// Converts a `toml::ser::Error` into the application's `Error::FormatError` variant.
    ///
    /// # Arguments
    /// * `value` - The `toml::ser::Error` to convert.
    ///
    /// # Returns
    /// An `Error::FormatError` variant.
    fn from(value: toml::ser::Error) -> Self {
        Self::FormatError(value.to_string())
    }
}

impl From<url::ParseError> for Error {
    /// Converts a `url::ParseError` into the application's `Error::FormatError` variant.
    ///
    /// # Arguments
    /// * `err` - The `url::ParseError` to convert.
    ///
    /// # Returns
    /// An `Error::FormatError` variant.
    fn from(err: url::ParseError) -> Self {
        Self::FormatError(err.to_string())
    }
}

impl From<chrono::ParseError> for Error {
    /// Converts a `chrono::ParseError` into the application's `Error::FormatError` variant.
    ///
    /// # Arguments
    /// * `err` - The `chrono::ParseError` to convert.
    ///
    /// # Returns
    /// An `Error::FormatError` variant.
    fn from(err: chrono::ParseError) -> Self {
        Self::FormatError(err.to_string())
    }
}

impl From<sqlx::Error> for Error {
    /// Converts a `sqlx::Error` into the application's `Error::DbError` variant.
    ///
    /// # Arguments
    /// * `err` - The `sqlx::Error` to convert.
    ///
    /// # Returns
    /// An `Error::DbError` variant.
    fn from(err: sqlx::Error) -> Self {
        Self::DbError(err.to_string())
    }
}
