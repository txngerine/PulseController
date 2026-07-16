use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

#[derive(Debug)]
pub struct UdpTransport {
    config: TransportConfig,
    state: TransportState,
    socket: Option<UdpSocket>,
    remote_addr: Option<SocketAddr>,
    local_addr: Option<SocketAddr>,
}

impl UdpTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: TransportState::Disconnected,
            socket: None,
            remote_addr: None,
            local_addr: None,
        }
    }

    pub fn remote_address(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    async fn reconnect(&mut self) -> Result<()> {
        if let Some(addr) = self.remote_addr {
            warn!("attempting reconnect to {}", addr);
            self.state = TransportState::Reconnecting;

            for attempt in 0..self.config.reconnect_attempts {
                match self.connect_internal(addr.ip().to_string().as_str(), addr.port()).await {
                    Ok(()) => {
                        info!("reconnected on attempt {}", attempt + 1);
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("reconnect attempt {} failed: {}", attempt + 1, e);
                        tokio::time::sleep(std::time::Duration::from_millis(
                            self.config.reconnect_delay_ms,
                        ))
                        .await;
                    }
                }
            }

            error!("reconnect failed after {} attempts", self.config.reconnect_attempts);
            self.state = TransportState::Error;
            Err(TransportError::ConnectionTimeout)
        } else {
            Err(TransportError::NotConnected)
        }
    }

    async fn connect_internal(&mut self, address: &str, port: u16) -> Result<()> {
        let bind_addr: SocketAddr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| TransportError::AddressParse(e.to_string()))?;

        let socket = UdpSocket::bind(bind_addr).await?;
        socket.connect(format!("{}:{}", address, port)).await?;

        let remote = format!("{}:{}", address, port)
            .parse::<SocketAddr>()
            .map_err(|e| TransportError::AddressParse(e.to_string()))?;

        self.local_addr = socket.local_addr().ok();
        self.remote_addr = Some(remote);
        self.socket = Some(socket);
        self.state = TransportState::Connected;

        info!("UDP connected to {} from {}", remote, self.local_addr.unwrap());
        Ok(())
    }
}

#[async_trait]
impl Transport for UdpTransport {
    fn name(&self) -> &str {
        "UDP"
    }

    fn state(&self) -> TransportState {
        self.state
    }

    fn config(&self) -> &TransportConfig {
        &self.config
    }

    async fn connect(&mut self, address: &str, port: u16) -> Result<()> {
        if self.state == TransportState::Connected {
            return Err(TransportError::AlreadyConnected);
        }
        self.connect_internal(address, port).await
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.socket = None;
        self.remote_addr = None;
        self.local_addr = None;
        self.state = TransportState::Disconnected;
        info!("UDP disconnected");
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let socket = self
            .socket
            .as_ref()
            .ok_or(TransportError::NotConnected)?;

        match socket.send(data).await {
            Ok(n) => {
                debug!("UDP sent {} bytes", n);
                Ok(n)
            }
            Err(e) => {
                error!("UDP send error: {}", e);
                self.state = TransportState::Error;
                Err(TransportError::Send(e.to_string()))
            }
        }
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let socket = self
            .socket
            .as_ref()
            .ok_or(TransportError::NotConnected)?;

        let mut buf = vec![0u8; self.config.buffer_size];

        match tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            socket.recv(&mut buf),
        )
        .await
        {
            Ok(Ok(n)) => {
                buf.truncate(n);
                debug!("UDP received {} bytes", n);
                Ok(buf)
            }
            Ok(Err(e)) => {
                error!("UDP receive error: {}", e);
                Err(TransportError::Receive(e.to_string()))
            }
            Err(_) => {
                warn!("UDP receive timeout");
                Err(TransportError::ConnectionTimeout)
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.state == TransportState::Connected
    }

    fn local_address(&self) -> Option<String> {
        self.local_addr.map(|a| a.to_string())
    }
}
