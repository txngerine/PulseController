use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl fmt::Display for TransportState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Reconnecting => write!(f, "Reconnecting"),
            Self::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportEvent {
    Connected { address: String },
    Disconnected { reason: String },
    DataReceived(Vec<u8>),
    LatencyUpdate { microseconds: u64 },
    PacketLoss { percentage: f64 },
    Error(String),
    StateChanged(TransportState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub bind_address: String,
    pub port: u16,
    pub buffer_size: usize,
    pub timeout_ms: u64,
    pub reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 9876,
            buffer_size: 4096,
            timeout_ms: 5000,
            reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
        }
    }
}

#[async_trait]
pub trait Transport: Send + Sync + fmt::Debug {
    fn name(&self) -> &str;
    fn state(&self) -> TransportState;
    fn config(&self) -> &TransportConfig;

    async fn connect(&mut self, address: &str, port: u16) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<usize>;
    async fn receive(&mut self) -> Result<Vec<u8>>;

    fn is_connected(&self) -> bool {
        self.state() == TransportState::Connected
    }

    fn local_address(&self) -> Option<String> {
        None
    }
}
