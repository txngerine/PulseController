use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("device not paired")]
    NotPaired,

    #[error("pairing failed: {0}")]
    PairingFailed(String),

    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("session expired")]
    SessionExpired,

    #[error("invalid token")]
    InvalidToken,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("crypto error: {0}")]
    Crypto(String),
}

pub type Result<T> = std::result::Result<T, SecurityError>;
