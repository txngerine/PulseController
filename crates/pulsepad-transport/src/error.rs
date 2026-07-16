use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("not connected")]
    NotConnected,

    #[error("already connected")]
    AlreadyConnected,

    #[error("connection refused")]
    ConnectionRefused,

    #[error("connection timeout")]
    ConnectionTimeout,

    #[error("address parse error: {0}")]
    AddressParse(String),

    #[error("send error: {0}")]
    Send(String),

    #[error("receive error: {0}")]
    Receive(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("device not found")]
    DeviceNotFound,

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("transport closed")]
    Closed,
}

pub type Result<T> = std::result::Result<T, TransportError>;
