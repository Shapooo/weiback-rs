#[allow(unused)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error {0}")]
    FromDataBase(#[from] sqlx::Error),
    #[error("reqwest error {0}")]
    FromReqwest(#[from] reqwest::Error),
    #[error("serde_json parse error {0}")]
    JsonParseFailed(#[from] serde_json::Error),
    #[error("tera error {0}")]
    FromTera(#[from] tera::Error),
    #[error("export error {0}")]
    IOFailed(#[from] std::io::Error),
    #[error("malformatted data {0}")]
    MalFormat(String),
    #[error("resource download failed {0}")]
    ResourceGetFailed(String),
    #[error("invalid cookie {0}")]
    InvalidCookie(#[from] reqwest::header::InvalidHeaderValue),
    #[error("unexpected error {0}")]
    Other(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
