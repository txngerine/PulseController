use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("deserialization error: {0}")]
    Deserialization(String),

    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u16),

    #[error("checksum mismatch: expected {expected:#06x}, got {actual:#06x}")]
    ChecksumMismatch { expected: u32, actual: u32 },

    #[error("packet too small: {0} bytes")]
    PacketTooSmall(usize),

    #[error("packet too large: {0} bytes")]
    PacketTooLarge(usize),

    #[error("invalid packet type: {0}")]
    InvalidPacketType(u8),

    #[error("sequence number out of order: expected {expected}, got {actual}")]
    SequenceOutOfOrder { expected: u64, actual: u64 },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<bincode::Error> for ProtocolError {
    fn from(e: bincode::Error) -> Self {
        ProtocolError::Serialization(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ProtocolError>;
