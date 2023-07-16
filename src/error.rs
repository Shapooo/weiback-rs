#[allow(unused)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error {0}")]
    DataBaseError(#[from] sqlx::Error),
    #[error("reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("serde_json parse error {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("tera error {0}")]
    TeraError(#[from] tera::Error),
    #[error("export error {0}")]
    ExportError(#[from] std::io::Error),
    #[error("malformatted data {0}")]
    MalFormat(String),
    #[error("resource download failed {0}")]
    ResourceGetFailed(String),
    #[error("invalid cookie {0}")]
    InvalidCookie(#[from] reqwest::header::InvalidHeaderValue),
    #[error("unexpected error {0}")]
    UnexpectedError(&'static str),
    #[error("resource not in local")]
    NotInLocal,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
