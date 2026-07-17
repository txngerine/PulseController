use std::fmt;
use tracing::{debug, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

/// Transport priority for fallback ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportKind {
    Usb,
    Bluetooth,
    Udp,
    Quic,
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb => write!(f, "USB"),
            Self::Bluetooth => write!(f, "Bluetooth"),
            Self::Udp => write!(f, "UDP"),
            Self::Quic => write!(f, "QUIC"),
        }
    }
}

/// Manages multiple transports with automatic fallback.
///
/// When connected, uses the highest-priority transport. If it fails,
/// automatically falls back to the next available transport.
#[derive(Debug)]
pub struct TransportManager {
    config: TransportConfig,
    active: Option<Box<dyn Transport>>,
    active_kind: Option<TransportKind>,
    fallback_order: Vec<TransportKind>,
    device_address: Option<String>,
    device_port: u16,
    connected: bool,
}

impl TransportManager {
    pub fn new(config: TransportConfig) -> Self {
        let fallback_order = vec![
            TransportKind::Usb,
            TransportKind::Bluetooth,
            TransportKind::Udp,
            TransportKind::Quic,
        ];

        Self {
            config,
            active: None,
            active_kind: None,
            fallback_order,
            device_address: None,
            device_port: 0,
            connected: false,
        }
    }

    /// Set a custom fallback priority order (highest first).
    pub fn set_fallback_order(&mut self, order: Vec<TransportKind>) {
        self.fallback_order = order;
    }

    /// Get the kind of the currently active transport.
    pub fn active_kind(&self) -> Option<TransportKind> {
        self.active_kind
    }

    /// Get access to the active transport, if any.
    pub fn active(&self) -> Option<&Box<dyn Transport>> {
        self.active.as_ref()
    }

    pub fn active_mut(&mut self) -> Option<&mut Box<dyn Transport>> {
        self.active.as_mut()
    }

    /// Try to connect using any transport, starting from highest priority.
    pub async fn connect(&mut self, address: &str, port: u16) -> Result<()> {
        self.device_address = Some(address.to_string());
        self.device_port = port;

        for kind in &self.fallback_order {
            match self.try_transport(*kind, address, port).await {
                Ok(t) => {
                    info!("connected via {kind}");
                    self.active = Some(t);
                    self.active_kind = Some(*kind);
                    self.connected = true;
                    return Ok(());
                }
                Err(e) => {
                    warn!("{kind} failed: {e}, trying next transport...");
                }
            }
        }

        self.connected = false;
        Err(TransportError::Protocol(
            "all transports failed to connect".to_string(),
        ))
    }

    async fn try_transport(
        &self,
        kind: TransportKind,
        address: &str,
        port: u16,
    ) -> Result<Box<dyn Transport>> {
        let mut transport: Box<dyn Transport> = match kind {
            TransportKind::Udp => Box::new(crate::udp::UdpTransport::new(self.config.clone())),
            TransportKind::Bluetooth => {
                Box::new(crate::bluetooth::BluetoothTransport::new(self.config.clone()))
            }
            TransportKind::Usb => Box::new(crate::usb::UsbTransport::new(self.config.clone())),
            TransportKind::Quic => Box::new(crate::quic::QuicTransport::new(self.config.clone())),
        };

        transport.connect(address, port).await?;
        Ok(transport)
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut t) = self.active {
            t.disconnect().await?;
        }
        self.active = None;
        self.active_kind = None;
        self.connected = false;
        info!("all transports disconnected");
        Ok(())
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        match self.active.as_mut() {
            Some(t) => match t.send(data).await {
                Ok(n) => Ok(n),
                Err(e) => {
                    warn!("active transport send failed: {e}, attempting fallback...");
                    self.fallback().await;
                    match self.active.as_mut() {
                        Some(t) => t.send(data).await,
                        None => Err(TransportError::NotConnected),
                    }
                }
            },
            None => Err(TransportError::NotConnected),
        }
    }

    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        match self.active.as_mut() {
            Some(t) => match t.receive().await {
                Ok(data) => Ok(data),
                Err(e) => {
                    debug!("active transport receive failed: {e}, attempting fallback...");
                    self.fallback().await;
                    match self.active.as_mut() {
                        Some(t) => t.receive().await,
                        None => Err(TransportError::NotConnected),
                    }
                }
            },
            None => Err(TransportError::NotConnected),
        }
    }

    async fn fallback(&mut self) {
        let current = self.active_kind;
        let addr = match &self.device_address {
            Some(a) => a.clone(),
            None => return,
        };
        let port = self.device_port;

        for kind in &self.fallback_order {
            if Some(*kind) == current {
                continue;
            }
            match self.try_transport(*kind, &addr, port).await {
                Ok(t) => {
                    info!("fallback to {kind} successful");
                    self.active = Some(t);
                    self.active_kind = Some(*kind);
                    return;
                }
                Err(e) => {
                    warn!("fallback to {kind} failed: {e}");
                }
            }
        }

        warn!("no fallback transport available");
        self.active = None;
        self.active_kind = None;
        self.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn state(&self) -> TransportState {
        match self.active.as_ref() {
            Some(t) => t.state(),
            None => TransportState::Disconnected,
        }
    }
}
