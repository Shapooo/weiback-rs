#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    DataBaseError(#[from] sqlx::Error),
    #[error("reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("serde_json parse error")]
    JsonParseError(#[from] serde_json::Error),
    #[error("tera error")]
    TeraError(#[from] tera::Error),
    #[error("malformatted data")]
    MalFormat(&'static str),
    #[error("resource download failed")]
    ResourceGetFailed(&'static str),
    #[error("unexpected error")]
    UnexpectedError(&'static str),
    #[error("resource not in local")]
    NotInLocal,
    #[error("export error")]
    ExportError(#[from] std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
