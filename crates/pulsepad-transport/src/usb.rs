use async_trait::async_trait;
use nusb::Interface;
use nusb::transfer::RequestBuffer;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

const PULSEPAD_VID: u16 = 0x1209;
const PULSEPAD_PID: u16 = 0x0001;
const PULSEPAD_OUT_EP: u8 = 0x02;
const PULSEPAD_IN_EP: u8 = 0x81;

pub struct UsbTransport {
    config: TransportConfig,
    state: TransportState,
    interface: Option<Arc<Mutex<Interface>>>,
    connected_device: Option<String>,
}

impl fmt::Debug for UsbTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UsbTransport")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("connected_device", &self.connected_device)
            .finish()
    }
}

impl UsbTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: TransportState::Disconnected,
            interface: None,
            connected_device: None,
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
        info!("connecting to USB device: {address}");

        let device = nusb::list_devices()
            .map_err(|e| TransportError::Protocol(format!("list USB devices: {e}")))?
            .into_iter()
            .find(|d| {
                let vid = d.vendor_id();
                let pid = d.product_id();
                (vid == PULSEPAD_VID && pid == PULSEPAD_PID)
                    || d.product_string()
                        .map(|s| s.contains("PulsePad") || s.contains(address))
                        .unwrap_or(false)
            })
            .ok_or(TransportError::DeviceNotFound)?;

        info!("found USB device: {:04x}:{:04x} {}",
            device.vendor_id(), device.product_id(),
            device.product_string().unwrap_or_default());

        let device_info = device.open()
            .map_err(|e| TransportError::Protocol(format!("open USB device: {e}")))?;

        let interface = device_info.detach_and_claim_interface(0)
            .map_err(|e| TransportError::Protocol(format!("claim USB interface: {e}")))?;

        self.interface = Some(Arc::new(Mutex::new(interface)));
        self.connected_device = Some(format!("USB:{:04x}:{:04x}", PULSEPAD_VID, PULSEPAD_PID));
        self.state = TransportState::Connected;
        info!("USB connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.interface = None;
        self.connected_device = None;
        self.state = TransportState::Disconnected;
        info!("USB disconnected");
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let iface = self.interface.as_ref()
            .ok_or(TransportError::NotConnected)?;
        let guard = iface.lock().await;

        let completion = guard.bulk_out(PULSEPAD_OUT_EP, data.to_vec()).await;
        completion.into_result()
            .map_err(|e| TransportError::Send(format!("USB send: {e}")))?;

        Ok(data.len())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let iface = self.interface.as_ref()
            .ok_or(TransportError::NotConnected)?;
        let guard = iface.lock().await;

        let buf = RequestBuffer::new(self.config.buffer_size);
        let completion = guard.bulk_in(PULSEPAD_IN_EP, buf).await;
        let received = completion.into_result()
            .map_err(|e| TransportError::Receive(format!("USB receive: {e}")))?;

        let data = received.to_vec();
        debug!("USB received {} bytes", data.len());
        Ok(data)
    }

    fn local_address(&self) -> Option<String> {
        self.connected_device.clone()
    }
}
