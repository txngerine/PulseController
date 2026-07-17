use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

use pulsepad_protocol::packet::{Packet, PacketType};
use pulsepad_protocol::wire::{self, WireFrame, StreamId, PacketFlags};
use pulsepad_transport::traits::{Transport, TransportConfig, TransportState};
use pulsepad_transport::udp::UdpTransport;

#[derive(Debug)]
pub struct ConnectionManager {
    transport: RwLock<Option<Box<dyn Transport>>>,
    config: TransportConfig,
    sequence: RwLock<u32>,
    auth_secret: RwLock<Option<[u8; 32]>>,
}

impl ConnectionManager {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            transport: RwLock::new(None),
            config,
            sequence: RwLock::new(0),
            auth_secret: RwLock::new(None),
        }
    }

    pub fn set_auth_secret(&self, secret: [u8; 32]) {
        *self.auth_secret.write() = Some(secret);
    }

    fn next_seq(&self) -> u32 {
        let mut seq = self.sequence.write();
        let v = *seq;
        *seq = seq.wrapping_add(1);
        v
    }

    pub async fn connect(&self, address: &str, port: u16) -> anyhow::Result<()> {
        let mut transport = Box::new(UdpTransport::new(self.config.clone()));
        transport.connect(address, port).await?;

        *self.transport.write() = Some(transport);
        info!("connected to {}:{}", address, port);
        Ok(())
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        // Send disconnect frame
        if let Ok(frame) = self.send_inner(StreamId::Control, PacketFlags::FIN | PacketFlags::RELIABLE, &[]) {
            let _ = frame;
        }
        if let Some(mut transport) = self.transport.write().take() {
            transport.disconnect().await?;
        }
        info!("disconnected");
        Ok(())
    }

    pub async fn send_packet(&self, packet: &Packet) -> anyhow::Result<()> {
        let frame = wire::packet_to_frame(packet, self.next_seq())?;
        self.send_frame(&frame).await
    }

    pub async fn send_frame(&self, frame: &WireFrame) -> anyhow::Result<()> {
        let data = frame.encode();
        let mut transport = self.transport.write();
        match transport.as_mut() {
            Some(t) => {
                t.send(&data).await?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("not connected")),
        }
    }

    async fn send_inner(&self, stream_id: StreamId, flags: PacketFlags, payload: &[u8]) -> anyhow::Result<()> {
        let frame = WireFrame::new(stream_id, flags, self.next_seq(), payload.to_vec());
        self.send_frame(&frame).await
    }

    /// Send a handshake with optional auth secret.
    pub async fn send_handshake(&self, device_name: &str, device_id: &str) -> anyhow::Result<()> {
        use pulsepad_protocol::packet::HandshakePayload;
        let auth = *self.auth_secret.read();
        let payload = HandshakePayload {
            device_name: device_name.to_string(),
            device_id: uuid::Uuid::parse_str(device_id).unwrap_or_default(),
            protocol_version: pulsepad_protocol::version::ProtocolVersion::CURRENT.as_u32(),
            capabilities: pulsepad_protocol::packet::DeviceCapabilities {
                has_gyroscope: false,
                has_accelerometer: false,
                has_touchpad: true,
                has_rumble: false,
                max_battery_level: 100,
                supported_transports: vec![
                    pulsepad_protocol::packet::TransportType::Udp,
                ],
            },
            session_token: None,
            auth_secret: auth,
        };
        let packet = Packet::new(PacketType::Handshake, self.next_seq() as u64, bincode::serialize(&payload)?);
        self.send_packet(&packet).await
    }

    /// Receive a raw frame from the transport.
    pub async fn receive_frame(&self) -> anyhow::Result<WireFrame> {
        let mut transport = self.transport.write();
        match transport.as_mut() {
            Some(t) => {
                let data = t.receive().await?;
                let frame = WireFrame::decode(&data)?;

                // Validate auth if this is a control stream message
                if let Some(secret) = *self.auth_secret.read() {
                    if frame.stream_id == StreamId::Control && frame.is_syn() {
                        // Validate handshake contains matching auth
                        if let Ok(packet) = wire::frame_to_packet(&frame) {
                            if let Some(PacketType::Handshake) = packet.packet_type() {
                                if let Ok(payload) = bincode::deserialize::<pulsepad_protocol::packet::HandshakePayload>(&packet.payload) {
                                    if let Some(client_secret) = payload.auth_secret {
                                        if client_secret != secret {
                                            return Err(anyhow::anyhow!("auth secret mismatch"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(frame)
            }
            None => Err(anyhow::anyhow!("not connected")),
        }
    }

    /// Receive and unwrap a typed Packet (for Control stream messages).
    pub async fn receive_packet(&self) -> anyhow::Result<Packet> {
        let frame = self.receive_frame().await?;
        let packet = wire::frame_to_packet(&frame)?;
        Ok(packet)
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
