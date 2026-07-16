use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("network error: {0}")]
    Network(String),

    #[error("broadcast error: {0}")]
    Broadcast(String),

    #[error("mdns error: {0}")]
    Mdns(String),

    #[error("bluetooth error: {0}")]
    Bluetooth(String),
}

pub type Result<T> = std::result::Result<T, DiscoveryError>;
