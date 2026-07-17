use async_trait::async_trait;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::Peripheral;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

const PULSEPAD_SERVICE_UUID: &str = "0000FEE1-0000-1000-8000-00805F9B34FB";
const PULSEPAD_WRITE_CHAR_UUID: &str = "0000FEE2-0000-1000-8000-00805F9B34FB";
const PULSEPAD_NOTIFY_CHAR_UUID: &str = "0000FEE3-0000-1000-8000-00805F9B34FB";

#[derive(Debug)]
pub struct BluetoothTransport {
    config: TransportConfig,
    state: TransportState,
    peripheral: Option<Arc<Mutex<Peripheral>>>,
    write_char: Option<Characteristic>,
    notify_char: Option<Characteristic>,
    connected_device: Option<String>,
    rx_buffer: Arc<Mutex<Vec<u8>>>,
}

impl BluetoothTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: TransportState::Disconnected,
            peripheral: None,
            write_char: None,
            notify_char: None,
            connected_device: None,
            rx_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn ensure_adapter() -> Result<btleplug::platform::Adapter> {
        let manager = btleplug::platform::Manager::new().await
            .map_err(|e| TransportError::Protocol(format!("BLE manager init: {e}")))?;
        let adapters = manager.adapters().await
            .map_err(|e| TransportError::Protocol(format!("list adapters: {e}")))?;
        adapters.into_iter().next()
            .ok_or(TransportError::DeviceNotFound)
    }

    async fn find_device(adapter: &btleplug::platform::Adapter, address: &str) -> Result<Peripheral> {
        adapter.start_scan(ScanFilter::default()).await
            .map_err(|e| TransportError::Protocol(format!("scan start: {e}")))?;

        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        adapter.stop_scan().await.ok();

        let peripherals = adapter.peripherals().await
            .map_err(|e| TransportError::Protocol(format!("list peripherals: {e}")))?;

        for p in &peripherals {
            if let Some(props) = p.properties().await.ok().and_then(|x| x) {
                let name = props.local_name.unwrap_or_default();
                let id = p.id().to_string();
                debug!("  BLE device: {name} ({id})");
                if id.contains(address) || name.contains(address) || name.contains("PulsePad") {
                    info!("found target BLE device: {name} ({id})");
                    return Ok(p.clone());
                }
            }
        }
        Err(TransportError::DeviceNotFound)
    }

    async fn discover_characteristics(p: &Peripheral) -> Result<(Characteristic, Characteristic)> {
        p.discover_services().await
            .map_err(|e| TransportError::Protocol(format!("service discovery: {e}")))?;

        let services = p.services();
        let svc_uuid_upper = PULSEPAD_SERVICE_UUID.to_uppercase();
        let write_uuid_upper = PULSEPAD_WRITE_CHAR_UUID.to_uppercase();
        let notify_uuid_upper = PULSEPAD_NOTIFY_CHAR_UUID.to_uppercase();

        let mut write_char = None;
        let mut notify_char = None;

        for service in &services {
            if service.uuid.to_string().to_uppercase() == svc_uuid_upper {
                for ch in &service.characteristics {
                    let uuid = ch.uuid.to_string().to_uppercase();
                    if uuid == write_uuid_upper {
                        write_char = Some(ch.clone());
                    } else if uuid == notify_uuid_upper {
                        notify_char = Some(ch.clone());
                    }
                }
            }
        }

        let write = write_char.ok_or_else(|| TransportError::Protocol("write char not found".into()))?;
        let notify = notify_char.ok_or_else(|| TransportError::Protocol("notify char not found".into()))?;
        Ok((write, notify))
    }

    async fn setup_notifications(p: &Peripheral, notify_char: Characteristic, rx: Arc<Mutex<Vec<u8>>>) -> Result<()> {
        p.subscribe(&notify_char).await
            .map_err(|e| TransportError::Protocol(format!("subscribe char: {e}")))?;

        let mut notification_stream = p.notifications().await
            .map_err(|e| TransportError::Protocol(format!("notification stream: {e}")))?;

        tokio::spawn(async move {
            use futures::StreamExt;
            while let Some(data) = notification_stream.next().await {
                let mut buf = rx.lock().await;
                buf.extend_from_slice(&data.value);
            }
        });

        info!("BLE notifications enabled");
        Ok(())
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

        let adapter = Self::ensure_adapter().await?;
        let peripheral = Self::find_device(&adapter, address).await?;

        peripheral.connect().await
            .map_err(|e| TransportError::Protocol(format!("BLE connect: {e}")))?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let (write_char, notify_char) = Self::discover_characteristics(&peripheral).await?;
        Self::setup_notifications(&peripheral, notify_char.clone(), self.rx_buffer.clone()).await?;

        let device_id = peripheral.id().to_string();
        self.connected_device = Some(device_id.clone());
        self.write_char = Some(write_char);
        self.notify_char = Some(notify_char);
        self.peripheral = Some(Arc::new(Mutex::new(peripheral)));
        self.state = TransportState::Connected;
        info!("BLE connected to {device_id}");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref p) = self.peripheral {
            let p = p.lock().await;
            p.disconnect().await.ok();
        }
        self.peripheral = None;
        self.write_char = None;
        self.notify_char = None;
        self.connected_device = None;
        self.state = TransportState::Disconnected;
        info!("BLE disconnected");
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let p = self.peripheral.as_ref()
            .ok_or(TransportError::NotConnected)?;
        let ch = self.write_char.as_ref().ok_or(TransportError::NotConnected)?;
        let p = p.lock().await;
        p.write(ch, data, WriteType::WithoutResponse).await
            .map_err(|e| TransportError::Send(e.to_string()))?;
        Ok(data.len())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let mut buf = self.rx_buffer.lock().await;
        if buf.is_empty() {
            return Err(TransportError::ConnectionTimeout);
        }
        let data = buf.clone();
        buf.clear();
        Ok(data)
    }

    fn is_connected(&self) -> bool {
        self.state == TransportState::Connected
    }
}
