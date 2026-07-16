use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("collection error: {0}")]
    Collection(String),
}

pub type Result<T> = std::result::Result<T, TelemetryError>;
