use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("directory error: {0}")]
    Directory(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;
