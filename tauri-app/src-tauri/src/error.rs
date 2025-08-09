use tauri::ipc::InvokeError;

#[derive(Debug, Clone)]
pub struct Error(pub String);
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for InvokeError {
    fn from(err: Error) -> Self {
        Self(serde_json::Value::String(err.0))
    }
}

impl<E: std::error::Error> From<E> for Error {
    fn from(e: E) -> Self {
        Self(e.to_string())
    }
}

impl From<Error> for anyhow::Error {
    fn from(err: Error) -> Self {
        anyhow::Error::msg(err.0)
    }
}
