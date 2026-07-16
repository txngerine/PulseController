use async_trait::async_trait;
use tracing::{info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

#[derive(Debug)]
pub struct UsbTransport {
    config: TransportConfig,
    state: TransportState,
}

impl UsbTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: TransportState::Disconnected,
        }
    }
}

#[async_trait]
impl Transport for UsbTransport {
    fn name(&self) -> &str {
        "USB"
    }

    fn state(&self) -> TransportState {
        self.state
    }

    fn config(&self) -> &TransportConfig {
        &self.config
    }

    async fn connect(&mut self, address: &str, _port: u16) -> Result<()> {
        if self.state == TransportState::Connected {
            return Err(TransportError::AlreadyConnected);
        }

        self.state = TransportState::Connecting;
        info!("connecting to USB device: {}", address);

        warn!("USB transport is a placeholder - not yet implemented");
        self.state = TransportState::Error;
        Err(TransportError::Protocol(
            "USB transport not yet implemented".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.state = TransportState::Disconnected;
        info!("USB disconnected");
        Ok(())
    }

    async fn send(&mut self, _data: &[u8]) -> Result<usize> {
        Err(TransportError::NotConnected)
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        Err(TransportError::NotConnected)
    }
}
