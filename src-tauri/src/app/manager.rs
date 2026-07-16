use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use pulsepad_platform::traits::BackendConfig;
use uuid::Uuid;

use pulsepad_discovery::DiscoveryManager;
use pulsepad_input::InputEngine;
use pulsepad_platform::traits::InputBackend;
use pulsepad_protocol::packet::{InputPayload, Packet, PacketType, RawPacket, parse_raw_packet};
use pulsepad_profiles::ProfileManager;
use pulsepad_security::PairingManager;
use pulsepad_storage::StorageManager;
use pulsepad_telemetry::TelemetryCollector;
use pulsepad_transport::traits::{Transport, TransportConfig};
use pulsepad_transport::udp::UdpTransport;
use crate::commands::logs::LogStore;

#[derive(Debug)]
pub struct AppManagerInner {
    pub storage: RwLock<StorageManager>,
    pub profiles: RwLock<ProfileManager>,
    pub security: RwLock<PairingManager>,
    pub telemetry: RwLock<TelemetryCollector>,
    pub discovery: RwLock<DiscoveryManager>,
    pub input_engine: RwLock<InputEngine>,
    pub transport: RwLock<Option<Box<dyn Transport>>>,
    pub backend: RwLock<Option<Box<dyn InputBackend>>>,
    pub connected_device: RwLock<Option<ConnectedDevice>>,
    pub sequence_number: RwLock<u64>,
    pub log_store: LogStore,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectedDevice {
    pub id: String,
    pub name: String,
    pub address: String,
    pub transport: String,
    pub connected_at: String,
}

#[derive(Debug, Clone)]
pub struct AppManager {
    pub inner: Arc<AppManagerInner>,
}

impl AppManager {
    pub fn new() -> Self {
        let storage = StorageManager::new("pulsepad").expect("failed to create storage manager");
        let profiles = ProfileManager::new(storage.profiles_dir());
        let security = PairingManager::new(storage.data_dir().join("pairing.json"));
        let telemetry = TelemetryCollector::new();
        let discovery = DiscoveryManager::new(9877);
        let input_engine = InputEngine::new();

        Self {
            inner: Arc::new(AppManagerInner {
                storage: RwLock::new(storage),
                profiles: RwLock::new(profiles),
                security: RwLock::new(security),
                telemetry: RwLock::new(telemetry),
                discovery: RwLock::new(discovery),
                input_engine: RwLock::new(input_engine),
                transport: RwLock::new(None),
                backend: RwLock::new(None),
                connected_device: RwLock::new(None),
                sequence_number: RwLock::new(0),
                log_store: LogStore::default(),
            }),
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        info!("initializing app manager");

        {
            let mut storage = self.inner.storage.write().await;
            storage.initialize().await?;
        }

        {
            let mut security = self.inner.security.write().await;
            security.initialize().await?;
        }

        {
            let mut profiles = self.inner.profiles.write().await;
            profiles.load_all().await?;
        }

        #[cfg(target_os = "windows")]
        {
            let mut backend = pulsepad_platform::windows::WindowsBackend::new();
            backend.initialize(BackendConfig::default()).await
                .map_err(|e| anyhow::anyhow!("windows backend init failed: {e}"))?;
            *self.inner.backend.write().await = Some(Box::new(backend) as Box<dyn InputBackend>);
        }

        #[cfg(target_os = "macos")]
        {
            let mut backend = pulsepad_platform::macos::MacosBackend::new();
            backend.initialize(BackendConfig::default()).await
                .map_err(|e| anyhow::anyhow!("macos backend init failed: {e}"))?;
            *self.inner.backend.write().await = Some(Box::new(backend) as Box<dyn InputBackend>);
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            error!("unsupported platform");
            return Err(anyhow::anyhow!("unsupported platform"));
        }

        info!("app manager initialized");
        Ok(())
    }

    pub async fn connect_device(&self, address: &str, port: u16) -> anyhow::Result<()> {
        info!("connecting to device at {}:{}", address, port);

        let config = TransportConfig {
            port,
            ..Default::default()
        };

        let mut transport = Box::new(UdpTransport::new(config));
        transport.connect(address, port).await?;

        *self.inner.transport.write().await = Some(transport);

        let device = ConnectedDevice {
            id: Uuid::new_v4().to_string(),
            name: "Unknown Device".to_string(),
            address: address.to_string(),
            transport: "UDP".to_string(),
            connected_at: chrono::Utc::now().to_rfc3339(),
        };

        *self.inner.connected_device.write().await = Some(device);
        info!("connected to device");

        let manager = self.clone();
        tokio::spawn(async move {
            manager.input_processing_loop().await;
        });

        Ok(())
    }

    pub async fn disconnect_device(&self) -> anyhow::Result<()> {
        let transport_opt = self.inner.transport.write().await.take();
        if let Some(mut transport) = transport_opt {
            transport.disconnect().await?;
        }

        *self.inner.connected_device.write().await = None;
        info!("disconnected from device");
        Ok(())
    }

    pub async fn get_connection_status(&self) -> Option<String> {
        self.inner
            .connected_device
            .read()
            .await
            .as_ref()
            .map(|d| serde_json::to_string(d).unwrap_or_default())
    }

    pub fn log_store(&self) -> &LogStore {
        &self.inner.log_store
    }

    async fn input_processing_loop(&self) {
        info!("started input processing loop");

        loop {
            // Check if transport still exists
            {
                let transport = self.inner.transport.read().await;
                if transport.is_none() {
                    break;
                }
                let is_connected = transport
                    .as_ref()
                    .map(|t| t.is_connected())
                    .unwrap_or(false);
                if !is_connected {
                    break;
                }
            }

            // Receive data from transport
            let data = {
                let mut guard = self.inner.transport.write().await;
                match guard.as_mut() {
                    Some(t) => match t.receive().await {
                        Ok(data) => data,
                        Err(e) => {
                            warn!("receive error: {}", e);
                            self.inner.log_store.log("warn", &format!("receive error: {e}"));
                            continue;
                        }
                    },
                    None => break,
                }
            };

            // Parse packet — try framed protocol first, then raw Flutter packets
            let packet_result = Packet::deserialize(&data);

            match packet_result {
                Ok(packet) => {
                    self.inner.telemetry.read().await.record_packet_received();

                    match packet.packet_type() {
                        Some(PacketType::Input) => {
                            if let Ok(input) =
                                bincode::deserialize::<InputPayload>(&packet.payload)
                            {
                                self.process_controller_input(input).await;
                            }
                        }
                        Some(PacketType::Heartbeat) => {
                            debug!("received heartbeat");
                        }
                        Some(PacketType::Battery) => {
                            if let Ok(battery) = bincode::deserialize::<
                                pulsepad_protocol::packet::BatteryPayload,
                            >(&packet.payload)
                            {
                                info!(
                                    "battery: {}% {}",
                                    battery.level,
                                    if battery.charging { "charging" } else { "" }
                                );
                            }
                        }
                        _ => {
                            debug!("unhandled packet type: {:?}", packet.packet_type());
                        }
                    }
                }
                Err(_) => {
                    // Framed protocol failed — try raw Flutter packet format
                    if let Some(raw) = parse_raw_packet(&data) {
                        self.process_raw_packet(raw).await;
                    } else {
                        warn!("unrecognised packet ({} bytes)", data.len());
                        self.inner.log_store.log("warn", &format!("unrecognised packet ({} bytes)", data.len()));
                        self.inner.telemetry.read().await.record_packet_dropped();
                    }
                }
            }
        }

        info!("input processing loop ended");
    }

    /// Process a framed InputPayload from the binary protocol.
    async fn process_controller_input(&self, input: InputPayload) {
        let controller_input = pulsepad_input::ControllerInput::from(input);

        let backend_info = {
            let backend = self.inner.backend.read().await;
            backend.is_some()
        };

        if backend_info {
            let mut input_engine = self.inner.input_engine.write().await;
            let backend = self.inner.backend.read().await;

            if let Some(backend) = backend.as_ref() {
                if let Err(e) = input_engine
                    .process_input(&controller_input, backend.as_ref())
                    .await
                {
                    warn!("input processing error: {}", e);
                }
            }
        }
    }

    /// Process a raw 8-byte packet from the Flutter mobile app.
    async fn process_raw_packet(&self, raw: RawPacket) {
        match raw {
            RawPacket::Controller {
                left_x, left_y, right_x, right_y,
                trigger_l, trigger_r, buttons,
            } => {
                // Convert u8 sticks (0-255) to i16 (-32768..32767)
                let lx = ((left_x as i16) - 128) * 256;
                let ly = ((left_y as i16) - 128) * 256;
                let rx = ((right_x as i16) - 128) * 256;
                let ry = ((right_y as i16) - 128) * 256;

                let payload = InputPayload {
                    buttons: buttons as u32,
                    left_stick: pulsepad_protocol::packet::StickAxis { x: lx, y: ly },
                    right_stick: pulsepad_protocol::packet::StickAxis { x: rx, y: ry },
                    left_trigger: trigger_l,
                    right_trigger: trigger_r,
                    dpad: pulsepad_protocol::packet::DpadState {
                        up: buttons & 0x10 != 0,
                        down: buttons & 0x20 != 0,
                        left: buttons & 0x40 != 0,
                        right: buttons & 0x80 != 0,
                    },
                };
                self.process_controller_input(payload).await;
            }

            RawPacket::TrackpadMove { dx, dy } => {
                debug!("raw trackpad move: dx={}, dy={}", dx, dy);
                let backend = self.inner.backend.read().await;
                if let Some(backend) = backend.as_ref() {
                    if let Err(e) = backend.send_mouse_move(dx as i32, dy as i32).await {
                        warn!("trackpad move failed: {}", e);
                    }
                }
            }

            RawPacket::TrackpadClick { button } => {
                let mb = match button {
                    1 => pulsepad_platform::traits::MouseButton::Left,
                    2 => pulsepad_platform::traits::MouseButton::Right,
                    3 => pulsepad_platform::traits::MouseButton::Middle,
                    _ => pulsepad_platform::traits::MouseButton::Left,
                };
                debug!("raw trackpad click: button={:?}", mb);
                let backend = self.inner.backend.read().await;
                if let Some(backend) = backend.as_ref() {
                    if let Err(e) = backend.send_mouse_button(mb, true).await {
                        warn!("trackpad click down failed: {}", e);
                    }
                    if let Err(e) = backend.send_mouse_button(mb, false).await {
                        warn!("trackpad click up failed: {}", e);
                    }
                }
            }

            RawPacket::TrackpadScroll { dx, dy } => {
                debug!("raw trackpad scroll: dx={}, dy={}", dx, dy);
                let backend = self.inner.backend.read().await;
                if let Some(backend) = backend.as_ref() {
                    if let Err(e) = backend.send_mouse_scroll(dx as i32, dy as i32).await {
                        warn!("trackpad scroll failed: {}", e);
                    }
                }
            }

            RawPacket::TrackpadZoom { direction } => {
                debug!("raw trackpad zoom: direction={}", direction);
                let backend = self.inner.backend.read().await;
                if let Some(backend) = backend.as_ref() {
                    let y = if direction > 0 { -3 } else { 3 };
                    if let Err(e) = backend.send_mouse_scroll(0, y).await {
                        warn!("trackpad zoom failed: {}", e);
                    }
                }
            }

            RawPacket::TrackpadSwipe { direction } => {
                debug!("raw trackpad swipe: direction={}", direction);
            }

            RawPacket::Keyboard { key_code } => {
                debug!("raw keyboard: key_code={}", key_code);
                let backend = self.inner.backend.read().await;
                if let Some(backend) = backend.as_ref() {
                    if let Err(e) = backend.send_key_press(key_code as u32, true).await {
                        warn!("keyboard key down failed: {}", e);
                    }
                    if let Err(e) = backend.send_key_press(key_code as u32, false).await {
                        warn!("keyboard key up failed: {}", e);
                    }
                }
            }

            RawPacket::Gyro { pitch, yaw, roll, inverted } => {
                debug!("raw gyro: pitch={}, yaw={}, roll={}, inverted={}", pitch, yaw, roll, inverted);
            }

            RawPacket::Clipboard(bytes) => {
                if let Ok(text) = String::from_utf8(bytes) {
                    info!("raw clipboard sync: {} bytes, preview: {:?}",
                        text.len(),
                        &text[..text.len().min(50)]
                    );
                }
            }
        }
    }
}

impl Default for AppManager {
    fn default() -> Self {
        Self::new()
    }
}
