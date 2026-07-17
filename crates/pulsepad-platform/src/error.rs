use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("platform not supported: {0}")]
    Unsupported(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("device error: {0}")]
    DeviceError(String),

    #[error("input injection failed: {0}")]
    InjectionFailed(String),

    #[error("backend not initialized")]
    NotInitialized,

    #[error("configuration error: {0}")]
    Configuration(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;
