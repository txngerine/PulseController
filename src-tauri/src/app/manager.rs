use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use pulsepad_platform::traits::BackendConfig;
use uuid::Uuid;

use pulsepad_discovery::{DiscoveryManager, MdnsResponder};
use pulsepad_input::InputEngine;
use pulsepad_platform::traits::InputBackend;
use pulsepad_protocol::packet::{
    HandshakePayload, HandshakeAckPayload, InputPayload, Packet, PacketType,
    RawPacket, parse_raw_packet, DeviceCapabilities, ClockSyncPayload,
    TransportType, ProfileEntry, ProfileListPayload,
};
use pulsepad_protocol::wire::{self, WireFrame, StreamId};
use pulsepad_profiles::ProfileManager;
use pulsepad_security::PairingManager;
use pulsepad_storage::StorageManager;
use pulsepad_telemetry::TelemetryCollector;
use pulsepad_transport::traits::TransportConfig;
use pulsepad_transport::manager::TransportManager;
use crate::commands::logs::LogStore;
use serde::Deserialize;
use base64::Engine as _;

/// JSON control frame envelope — mirrors PacketType values as `"type"` field.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum JsonControlFrame {
    #[serde(rename = "1")]
    Handshake(JsonHandshakePayload),
    #[serde(rename = "50")]
    ClockSync(JsonClockSyncPayload),
    #[serde(rename = "53")]
    ProfileList(JsonProfileListRequest),
}

#[derive(Debug, Deserialize)]
struct JsonHandshakePayload {
    device_name: String,
    device_id: String,
    #[serde(default)]
    protocol_version: u32,
    #[serde(default)]
    capabilities: Option<JsonDeviceCapabilities>,
    #[serde(default)]
    session_token: Option<String>,
    #[serde(default)]
    auth_secret: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonDeviceCapabilities {
    #[serde(default)]
    has_gyroscope: bool,
    #[serde(default)]
    has_accelerometer: bool,
    #[serde(default)]
    has_touchpad: bool,
    #[serde(default)]
    has_rumble: bool,
    #[serde(default)]
    max_battery_level: u8,
    #[serde(default)]
    supported_transports: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonClockSyncPayload {
    #[serde(default)]
    t1: u64,
    #[serde(default)]
    t2: u64,
    #[serde(default)]
    t3: u64,
}

#[derive(Debug, Deserialize)]
struct JsonProfileListRequest {}

#[derive(Debug)]
pub struct AppManagerInner {
    pub storage: RwLock<StorageManager>,
    pub profiles: RwLock<ProfileManager>,
    pub security: RwLock<PairingManager>,
    pub telemetry: RwLock<TelemetryCollector>,
    pub discovery: RwLock<DiscoveryManager>,
    pub input_engine: RwLock<InputEngine>,
    pub transport: RwLock<Option<TransportManager>>,
    pub backend: RwLock<Option<Box<dyn InputBackend>>>,
    pub connected_device: RwLock<Option<ConnectedDevice>>,
    pub frame_sequence: RwLock<u32>,
    #[allow(dead_code)]
    pub sequence_number: RwLock<u64>,
    pub log_store: LogStore,
    pub auth_secret: RwLock<Option<[u8; 32]>>,
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
                frame_sequence: RwLock::new(0),
                sequence_number: RwLock::new(0),
                log_store: LogStore::default(),
                auth_secret: RwLock::new(None),
            }),
        }
    }

    pub fn next_seq(&self) -> u32 {
        let mut seq = self.inner.frame_sequence.blocking_write();
        let v = *seq;
        *seq = seq.wrapping_add(1);
        v
    }

    pub async fn set_auth_secret(&self, secret: [u8; 32]) {
        *self.inner.auth_secret.write().await = Some(secret);
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

        // Start mDNS responder so mobile devices can discover us
        let device_id = Uuid::new_v4().to_string();
        let responder = MdnsResponder::new(
            "PulseController",
            &device_id,
            "pending", // cert fingerprint will be set after security init
            35769,
        );
        if let Err(e) = responder.start().await {
            warn!("mDNS responder failed to start: {}", e);
        } else {
            info!("mDNS responder started");
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

        #[cfg(target_os = "linux")]
        {
            let mut backend = pulsepad_platform::linux::LinuxBackend::new();
            backend.initialize(BackendConfig::default()).await
                .map_err(|e| anyhow::anyhow!("linux backend init failed: {e}"))?;
            *self.inner.backend.write().await = Some(Box::new(backend) as Box<dyn InputBackend>);
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
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

        let mut mgr = TransportManager::new(config);
        mgr.connect(address, port).await
            .map_err(|e| anyhow::anyhow!("all transports failed: {e}"))?;

        let transport_name = mgr.active_kind()
            .map(|k| k.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        *self.inner.transport.write().await = Some(mgr);

        let connected = ConnectedDevice {
            id: Uuid::new_v4().to_string(),
            name: "Unknown Device".to_string(),
            address: address.to_string(),
            transport: transport_name,
            connected_at: chrono::Utc::now().to_rfc3339(),
        };

        info!("connected via {}", connected.transport);
        *self.inner.connected_device.write().await = Some(connected);

        let mgr = self.clone();
        tokio::spawn(async move {
            mgr.input_processing_loop().await;
        });

        Ok(())
    }

    pub async fn disconnect_device(&self) -> anyhow::Result<()> {
        let transport_opt = self.inner.transport.write().await.take();
        if let Some(mut mgr) = transport_opt {
            mgr.disconnect().await?;
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
        info!("started input processing loop (WireFrame protocol)");

        loop {
            {
                let mgr = self.inner.transport.read().await;
                if mgr.is_none() {
                    break;
                }
                if !mgr.as_ref().map(|m| m.is_connected()).unwrap_or(false) {
                    break;
                }
            }

            let data = {
                let mut guard = self.inner.transport.write().await;
                match guard.as_mut() {
                    Some(mgr) => match mgr.receive().await {
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

            self.inner.telemetry.read().await.record_packet_received();

            // Try WireFrame protocol first
            if let Ok(frame) = WireFrame::decode(&data) {
                self.process_wire_frame(frame).await;
            } else {
                // Fallback: try old framed protocol
                let packet_result = Packet::deserialize(&data);
                match packet_result {
                    Ok(packet) => {
                        self.process_typed_packet(packet).await;
                    }
                    Err(_) => {
                        // Fallback: try raw Flutter packet format
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
        }

        info!("input processing loop ended");
    }

    /// Process a WireFrame packet. Supports all stream types.
    async fn process_wire_frame(&self, frame: WireFrame) {
        match frame.stream_id {
            StreamId::Control => {
                self.process_control_frame(frame).await;
            }
            StreamId::Gamepad => {
                // Direct gamepad input - legacy 8-byte format inside payload
                if let Some(raw) = parse_raw_packet(&frame.payload) {
                    self.process_raw_packet(raw).await;
                }
            }
            StreamId::Keyboard => {
                if let Some(raw) = parse_raw_packet(&frame.payload) {
                    self.process_raw_packet(raw).await;
                }
            }
            StreamId::Mouse => {
                if let Some(raw) = parse_raw_packet(&frame.payload) {
                    self.process_raw_packet(raw).await;
                }
            }
            StreamId::Gyroscope => {
                if let Some(raw) = parse_raw_packet(&frame.payload) {
                    self.process_raw_packet(raw).await;
                }
            }
            StreamId::Heartbeat => {
                debug!("heartbeat received (seq={})", frame.sequence);
            }
            StreamId::ClockSync => {
                debug!("clock sync received (seq={})", frame.sequence);
            }
            _ => {
                debug!("unhandled stream: {:?}", frame.stream_id);
            }
        }
    }

    /// Process Control stream frames (handshake, session management).
    async fn process_control_frame(&self, frame: WireFrame) {
        // Try bincode-encoded Packet first
        if let Ok(packet) = wire::frame_to_packet(&frame) {
            self.process_typed_packet(packet).await;
            return;
        }
        // Fallback: try JSON payload from Dart/Flutter clients
        if let Ok(text) = std::str::from_utf8(&frame.payload) {
            match serde_json::from_str::<JsonControlFrame>(text) {
                Ok(JsonControlFrame::Handshake(json_payload)) => {
                    let device_id = uuid::Uuid::parse_str(&json_payload.device_id).unwrap_or_default();
                    let capabilities = json_payload.capabilities.as_ref().map(|c| DeviceCapabilities {
                        has_gyroscope: c.has_gyroscope,
                        has_accelerometer: c.has_accelerometer,
                        has_touchpad: c.has_touchpad,
                        has_rumble: c.has_rumble,
                        max_battery_level: c.max_battery_level,
                        supported_transports: if c.supported_transports.contains(&"ble".to_string()) {
                            vec![TransportType::Bluetooth]
                        } else {
                            vec![TransportType::Udp]
                        },
                    }).unwrap_or(DeviceCapabilities {
                        has_gyroscope: false,
                        has_accelerometer: false,
                        has_touchpad: true,
                        has_rumble: false,
                        max_battery_level: 100,
                        supported_transports: vec![TransportType::Udp],
                    });
                    let auth_secret = json_payload.auth_secret.as_ref()
                        .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                        .and_then(|v| v.try_into().ok());
                    let session_token = json_payload.session_token.as_ref()
                        .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                        .and_then(|v| v.try_into().ok());
                    let payload = HandshakePayload {
                        device_name: json_payload.device_name,
                        device_id,
                        protocol_version: json_payload.protocol_version,
                        capabilities,
                        session_token,
                        auth_secret,
                    };
                    self.process_handshake_inner(payload).await;
                }
                Ok(JsonControlFrame::ClockSync(json_sync)) => {
                    let sync = ClockSyncPayload {
                        t1: json_sync.t1,
                        t2: 0,
                        t3: 0,
                    };
                    let response = sync.into_response();
                    if let Ok(data) = bincode::serialize(&response) {
                        let pkt = Packet::new(PacketType::ClockSyncResponse, self.next_seq() as u64, data);
                        if let Ok(resp_frame) = wire::packet_to_frame(&pkt, self.next_seq()) {
                            let encoded = resp_frame.encode();
                            let mut guard = self.inner.transport.write().await;
                            if let Some(t) = guard.as_mut() {
                                let _ = t.send(&encoded).await;
                            }
                        }
                    }
                }
                Ok(JsonControlFrame::ProfileList(_)) => {
                    let profiles = self.inner.profiles.read().await;
                    let all = profiles.list_profiles();
                    let entries: Vec<ProfileEntry> = all.iter().map(|p| ProfileEntry {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                        is_active: p.is_default,
                        game_name: None,
                        controller_type: p.description.clone(),
                        last_used: None,
                    }).collect();
                    let list = ProfileListPayload { profiles: entries };
                    if let Ok(data) = bincode::serialize(&list) {
                        let pkt = Packet::new(PacketType::ProfileListResponse, self.next_seq() as u64, data);
                        if let Ok(resp_frame) = wire::packet_to_frame(&pkt, self.next_seq()) {
                            let encoded = resp_frame.encode();
                            let mut guard = self.inner.transport.write().await;
                            if let Some(t) = guard.as_mut() {
                                let _ = t.send(&encoded).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("failed to deserialize JSON control frame: {}", e);
                }
            }
            return;
        }
        warn!("failed to decode control frame payload");
    }

    /// Process a typed Packet from the Control stream.
    async fn process_typed_packet(&self, packet: Packet) {
        match packet.packet_type() {
            Some(PacketType::Handshake) => {
                if let Ok(payload) = bincode::deserialize::<HandshakePayload>(&packet.payload) {
                    self.process_handshake_inner(payload).await;
                }
            }
            Some(PacketType::Heartbeat) => {
                debug!("heartbeat received");
            }
            Some(PacketType::ClockSync) => {
                if let Ok(sync_req) = bincode::deserialize::<pulsepad_protocol::packet::ClockSyncPayload>(&packet.payload) {
                    let response_payload = sync_req.into_response();
                    if let Ok(data) = bincode::serialize(&response_payload) {
                        let response_packet = Packet::new(PacketType::ClockSyncResponse, self.next_seq() as u64, data);
                        if let Ok(frame) = wire::packet_to_frame(&response_packet, self.next_seq()) {
                            let encoded = frame.encode();
                            let mut guard = self.inner.transport.write().await;
                            if let Some(t) = guard.as_mut() {
                                let _ = t.send(&encoded).await;
                            }
                        }
                    }
                }
            }
            Some(PacketType::ProfileList) => {
                let profiles = self.inner.profiles.read().await;
                let all = profiles.list_profiles();
                let entry_list: Vec<pulsepad_protocol::packet::ProfileEntry> = all.iter().map(|p| {
                    pulsepad_protocol::packet::ProfileEntry {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                        is_active: p.is_default,
                        game_name: None,
                        controller_type: p.description.clone(),
                        last_used: None,
                    }
                }).collect();
                let list = pulsepad_protocol::packet::ProfileListPayload { profiles: entry_list };
                if let Ok(data) = bincode::serialize(&list) {
                    let pkt = Packet::new(PacketType::ProfileListResponse, self.next_seq() as u64, data);
                    if let Ok(frame) = wire::packet_to_frame(&pkt, self.next_seq()) {
                        let encoded = frame.encode();
                        let mut guard = self.inner.transport.write().await;
                        if let Some(t) = guard.as_mut() {
                            let _ = t.send(&encoded).await;
                        }
                    }
                }
            }
            Some(PacketType::Input) => {
                if let Ok(input) = bincode::deserialize::<InputPayload>(&packet.payload) {
                    self.process_controller_input(input).await;
                }
            }
            Some(PacketType::Battery) => {
                if let Ok(battery) = bincode::deserialize::<pulsepad_protocol::packet::BatteryPayload>(&packet.payload) {
                    info!("battery: {}% {}", battery.level, if battery.charging { "charging" } else { "" });
                }
            }
            _ => {
                debug!("unhandled control packet type: {:?}", packet.packet_type());
            }
        }
    }

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

    async fn process_handshake_inner(&self, payload: HandshakePayload) {
        info!("handshake from '{}' (id={})", payload.device_name, payload.device_id);

        let is_authed = {
            let expected = self.inner.auth_secret.read().await;
            match (&payload.auth_secret, expected.as_ref()) {
                (Some(client), Some(server)) if client == server => true,
                (Some(_), Some(_)) => {
                    warn!("auth secret mismatch from '{}'", payload.device_name);
                    false
                }
                _ => true,
            }
        };

        if !is_authed {
            let ack = HandshakeAckPayload {
                accepted: false,
                server_name: "PulseController".to_string(),
                protocol_version: pulsepad_protocol::version::ProtocolVersion::CURRENT.as_u32(),
                auth_challenge: None,
                session_token: None,
                reject_reason: Some("auth secret mismatch".to_string()),
            };
            if let Ok(ack_data) = bincode::serialize(&ack) {
                let ack_packet = Packet::new(PacketType::HandshakeAck, self.next_seq() as u64, ack_data);
                if let Ok(ack_frame) = wire::packet_to_frame(&ack_packet, self.next_seq()) {
                    let data = ack_frame.encode();
                    let mut guard = self.inner.transport.write().await;
                    if let Some(t) = guard.as_mut() {
                        let _ = t.send(&data).await;
                    }
                }
            }
            return;
        }

        let session_token: [u8; 32] = rand::random();
        let ack = HandshakeAckPayload {
            accepted: true,
            server_name: "PulseController".to_string(),
            protocol_version: pulsepad_protocol::version::ProtocolVersion::CURRENT.as_u32(),
            auth_challenge: None,
            session_token: Some(session_token),
            reject_reason: None,
        };

        if let Ok(ack_data) = bincode::serialize(&ack) {
            let ack_packet = Packet::new(PacketType::HandshakeAck, self.next_seq() as u64, ack_data);
            if let Ok(ack_frame) = wire::packet_to_frame(&ack_packet, self.next_seq()) {
                let data = ack_frame.encode();
                let mut guard = self.inner.transport.write().await;
                if let Some(t) = guard.as_mut() {
                    if let Err(e) = t.send(&data).await {
                        warn!("failed to send handshake ack: {}", e);
                    } else {
                        info!("handshake accepted for '{}'", payload.device_name);
                    }
                }
            }
        }
    }

    async fn process_raw_packet(&self, raw: RawPacket) {
        match raw {
            RawPacket::Controller {
                left_x, left_y, right_x, right_y,
                trigger_l, trigger_r, buttons,
            } => {
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
