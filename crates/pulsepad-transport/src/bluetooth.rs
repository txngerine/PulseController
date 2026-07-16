use async_trait::async_trait;
use tracing::{info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

#[derive(Debug)]
pub struct BluetoothTransport {
    config: TransportConfig,
    state: TransportState,
    connected_device: Option<String>,
}

impl BluetoothTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: TransportState::Disconnected,
            connected_device: None,
        }
    }
}

#[async_trait]
impl Transport for BluetoothTransport {
    fn name(&self) -> &str {
        "Bluetooth"
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
        info!("connecting to Bluetooth device: {}", address);

        // TODO: Implement BLE connection using platform-specific APIs
        warn!("Bluetooth transport is a placeholder - not yet implemented");
        self.state = TransportState::Error;
        Err(TransportError::Protocol(
            "Bluetooth transport not yet implemented".to_string(),
        ))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected_device = None;
        self.state = TransportState::Disconnected;
        info!("Bluetooth disconnected");
        Ok(())
    }

    async fn send(&mut self, _data: &[u8]) -> Result<usize> {
        Err(TransportError::NotConnected)
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        Err(TransportError::NotConnected)
    }
}
