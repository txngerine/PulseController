use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{DiscoveryError, Result};
use pulsepad_protocol::packet::TransportType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub transport: TransportType,
    pub signal_strength: Option<i32>,
    pub os: Option<String>,
    pub last_seen: u64,
}

#[derive(Debug)]
pub struct DiscoveryManager {
    devices: Arc<DashMap<String, DiscoveredDevice>>,
    broadcast_port: u16,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl DiscoveryManager {
    pub fn new(broadcast_port: u16) -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
            broadcast_port,
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    pub async fn start_udp_broadcast(&self) -> Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.broadcast_port)).await?;
        socket.set_broadcast(true)?;

        let devices = self.devices.clone();
        let running = self.running.clone();

        *running.write().await = true;

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];

            loop {
                if !*running.read().await {
                    break;
                }

                match tokio::time::timeout(
                    std::time::Duration::from_secs(1),
                    socket.recv_from(&mut buf),
                )
                .await
                {
                    Ok(Ok((len, addr))) => {
                        if let Ok(device) = serde_json::from_slice::<DiscoveredDevice>(&buf[..len])
                        {
                            debug!("discovered device: {} at {}", device.name, addr);
                            devices.insert(device.id.clone(), device);
                        }
                    }
                    Ok(Err(e)) => {
                        error!("discovery recv error: {}", e);
                    }
                    Err(_) => {
                        // Timeout, continue loop
                    }
                }
            }
        });

        info!("started UDP discovery on port {}", self.broadcast_port);
        Ok(())
    }

    pub async fn send_broadcast(&self, device_info: &DiscoveredDevice) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;

        let data = serde_json::to_vec(device_info)?;

        // Broadcast to common ports
        let broadcast_addrs = [
            format!("255.255.255.255:{}", self.broadcast_port),
            format!("127.0.0.1:{}", self.broadcast_port),
        ];

        for addr in &broadcast_addrs {
            if let Ok(dest) = addr.parse::<SocketAddr>() {
                if let Err(e) = socket.send_to(&data, dest).await {
                    warn!("failed to send broadcast to {}: {}", addr, e);
                }
            }
        }

        Ok(())
    }

    pub fn stop(&self) {
        if let Ok(mut running) = self.running.try_write() {
            *running = false;
        }
        info!("stopped discovery");
    }

    pub fn get_device(&self, id: &str) -> Option<DiscoveredDevice> {
        self.devices.get(id).map(|d| d.clone())
    }

    pub fn list_devices(&self) -> Vec<DiscoveredDevice> {
        self.devices.iter().map(|d| d.clone()).collect()
    }

    pub fn clear_stale_devices(&self, max_age_secs: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.devices.retain(|_, device| {
            now.saturating_sub(device.last_seen) < max_age_secs
        });
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}
