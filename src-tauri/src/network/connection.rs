use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

use pulsepad_protocol::packet::{Packet, PacketType};
use pulsepad_transport::traits::{Transport, TransportConfig, TransportState};
use pulsepad_transport::udp::UdpTransport;

#[derive(Debug)]
pub struct ConnectionManager {
    transport: RwLock<Option<Box<dyn Transport>>>,
    config: TransportConfig,
}

impl ConnectionManager {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            transport: RwLock::new(None),
            config,
        }
    }

    pub async fn connect(&self, address: &str, port: u16) -> anyhow::Result<()> {
        let mut transport = Box::new(UdpTransport::new(self.config.clone()));
        transport.connect(address, port).await?;

        *self.transport.write() = Some(transport);
        info!("connected to {}:{}", address, port);
        Ok(())
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        if let Some(mut transport) = self.transport.write().take() {
            transport.disconnect().await?;
        }
        info!("disconnected");
        Ok(())
    }

    pub async fn send(&self, packet: &Packet) -> anyhow::Result<()> {
        let data = packet.serialize()?;
        let mut transport = self.transport.write();
        match transport.as_mut() {
            Some(t) => {
                t.send(&data).await?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("not connected")),
        }
    }

    pub async fn receive(&self) -> anyhow::Result<Packet> {
        let mut transport = self.transport.write();
        match transport.as_mut() {
            Some(t) => {
                let data = t.receive().await?;
                let packet = Packet::deserialize(&data)?;
                Ok(packet)
            }
            None => Err(anyhow::anyhow!("not connected")),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.transport
            .read()
            .as_ref()
            .map(|t| t.is_connected())
            .unwrap_or(false)
    }

    pub fn state(&self) -> TransportState {
        self.transport
            .read()
            .as_ref()
            .map(|t| t.state())
            .unwrap_or(TransportState::Disconnected)
    }
}
