use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::version::ProtocolVersion;

pub const MAX_PACKET_SIZE: usize = 4096;
pub const HEADER_SIZE: usize = std::mem::size_of::<PacketHeader>();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    Handshake = 0x01,
    HandshakeAck = 0x02,
    Heartbeat = 0x03,
    Ping = 0x04,
    Pong = 0x05,
    Disconnect = 0x06,
    DisconnectAck = 0x07,
    Input = 0x10,
    InputBatch = 0x11,
    Battery = 0x20,
    Gyroscope = 0x21,
    Touchpad = 0x22,
    Accelerometer = 0x23,
    ProfileSwitch = 0x30,
    ProfileSwitchAck = 0x31,
    DeviceInfo = 0x40,
    DeviceInfoAck = 0x41,
    Error = 0xFE,
    Reserved = 0xFF,
}

impl PacketType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Self::Handshake),
            0x02 => Some(Self::HandshakeAck),
            0x03 => Some(Self::Heartbeat),
            0x04 => Some(Self::Ping),
            0x05 => Some(Self::Pong),
            0x06 => Some(Self::Disconnect),
            0x07 => Some(Self::DisconnectAck),
            0x10 => Some(Self::Input),
            0x11 => Some(Self::InputBatch),
            0x20 => Some(Self::Battery),
            0x21 => Some(Self::Gyroscope),
            0x22 => Some(Self::Touchpad),
            0x23 => Some(Self::Accelerometer),
            0x30 => Some(Self::ProfileSwitch),
            0x31 => Some(Self::ProfileSwitchAck),
            0x40 => Some(Self::DeviceInfo),
            0x41 => Some(Self::DeviceInfoAck),
            0xFE => Some(Self::Error),
            0xFF => Some(Self::Reserved),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C, packed)]
pub struct PacketHeader {
    pub version: u32,
    pub packet_type: u8,
    pub sequence: u64,
    pub timestamp: u64,
    pub payload_len: u16,
    pub checksum: u32,
}

impl PacketHeader {
    pub fn new(packet_type: PacketType, sequence: u64, payload_len: u16) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            version: ProtocolVersion::CURRENT.as_u32(),
            packet_type: packet_type as u8,
            sequence,
            timestamp: now,
            payload_len,
            checksum: 0,
        }
    }

    pub fn compute_checksum(&self, payload: &[u8]) -> u32 {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.version.to_le_bytes());
        hasher.update([self.packet_type]);
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.payload_len.to_le_bytes());
        hasher.update(payload);
        let hash = hasher.finalize();
        u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
    }

    pub fn version(&self) -> ProtocolVersion {
        let major = (self.version >> 16) as u16;
        let minor = (self.version & 0xFFFF) as u16;
        ProtocolVersion::new(major, minor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(packet_type: PacketType, sequence: u64, payload: Vec<u8>) -> Self {
        let mut header = PacketHeader::new(packet_type, sequence, payload.len() as u16);
        header.checksum = header.compute_checksum(&payload);
        Self { header, payload }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, crate::error::ProtocolError> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        let header_bytes = bincode::serialize(&self.header)?;
        buf.extend_from_slice(&header_bytes);
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, crate::error::ProtocolError> {
        if data.len() < HEADER_SIZE {
            return Err(crate::error::ProtocolError::PacketTooSmall(data.len()));
        }
        if data.len() > MAX_PACKET_SIZE {
            return Err(crate::error::ProtocolError::PacketTooLarge(data.len()));
        }

        let header: PacketHeader = bincode::deserialize(&data[..HEADER_SIZE])?;

        let payload = data[HEADER_SIZE..HEADER_SIZE + header.payload_len as usize].to_vec();
        let expected_checksum = header.compute_checksum(&payload);

        if header.checksum != expected_checksum {
            return Err(crate::error::ProtocolError::ChecksumMismatch {
                expected: header.checksum,
                actual: expected_checksum,
            });
        }

        Ok(Self { header, payload })
    }

    pub fn packet_type(&self) -> Option<PacketType> {
        PacketType::from_u8(self.header.packet_type)
    }

    pub fn sequence(&self) -> u64 {
        self.header.sequence
    }

    pub fn timestamp(&self) -> u64 {
        self.header.timestamp
    }
}

/// Raw packet type from the Flutter mobile app.
/// Flutter sends 8-byte arrays where the first byte is a type prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RawPacketType {
    /// Trackpad move: [0xA1, dx, dy, ...]
    TrackpadMove = 0xA1,
    /// Trackpad click: [0xA2, button_code, ...]
    TrackpadClick = 0xA2,
    /// Trackpad scroll: [0xA3, dx, dy, ...]
    TrackpadScroll = 0xA3,
    /// Trackpad zoom: [0xA4, direction, ...]
    TrackpadZoom = 0xA4,
    /// Trackpad swipe: [0xA5, direction_code, ...]
    TrackpadSwipe = 0xA5,
    /// Keyboard: [0xB0, key_code, ...]
    Keyboard = 0xB0,
    /// Gyroscope: [0xC0, pitch_h, pitch_l, yaw_h, yaw_l, roll_h, roll_l, flags]
    Gyro = 0xC0,
    /// Clipboard: [0xD0, ...content_bytes]
    Clipboard = 0xD0,
}

impl RawPacketType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0xA1 => Some(Self::TrackpadMove),
            0xA2 => Some(Self::TrackpadClick),
            0xA3 => Some(Self::TrackpadScroll),
            0xA4 => Some(Self::TrackpadZoom),
            0xA5 => Some(Self::TrackpadSwipe),
            0xB0 => Some(Self::Keyboard),
            0xC0 => Some(Self::Gyro),
            0xD0 => Some(Self::Clipboard),
            _ => None,
        }
    }
}

/// Represents a parsed raw packet from the Flutter mobile app.
#[derive(Debug, Clone)]
pub enum RawPacket {
    /// Trackpad move with signed dx/dy deltas.
    TrackpadMove { dx: i8, dy: i8 },
    /// Trackpad click (1=left, 2=right, 3=middle).
    TrackpadClick { button: u8 },
    /// Trackpad scroll with signed dx/dy.
    TrackpadScroll { dx: i8, dy: i8 },
    /// Trackpad zoom direction (positive=in, negative=out).
    TrackpadZoom { direction: i8 },
    /// Trackpad swipe direction (0=up, 1=down, 2=left).
    TrackpadSwipe { direction: u8 },
    /// Keyboard key code.
    Keyboard { key_code: u8 },
    /// Gyroscope data: pitch, yaw, roll as i16 big-endian pairs.
    Gyro { pitch: i16, yaw: i16, roll: i16, inverted: bool },
    /// Clipboard content as UTF-8 bytes.
    Clipboard(Vec<u8>),
    /// Controller input: sticks (u8 0-255), triggers, button bitmask.
    Controller {
        left_x: u8,
        left_y: u8,
        right_x: u8,
        right_y: u8,
        trigger_l: u8,
        trigger_r: u8,
        buttons: u8,
    },
}

/// Try to parse data as a raw Flutter packet.
/// Returns None if the data doesn't look like a raw Flutter packet.
pub fn parse_raw_packet(data: &[u8]) -> Option<RawPacket> {
    if data.is_empty() {
        return None;
    }

    let type_byte = data[0];

    // If it matches a known raw type prefix, parse it as a typed raw packet
    if let Some(raw_type) = RawPacketType::from_u8(type_byte) {
        return match raw_type {
            RawPacketType::TrackpadMove => {
                if data.len() < 3 { return None; }
                Some(RawPacket::TrackpadMove {
                    dx: data[1] as i8,
                    dy: data[2] as i8,
                })
            }
            RawPacketType::TrackpadClick => {
                if data.len() < 2 { return None; }
                Some(RawPacket::TrackpadClick { button: data[1] })
            }
            RawPacketType::TrackpadScroll => {
                if data.len() < 3 { return None; }
                Some(RawPacket::TrackpadScroll {
                    dx: data[1] as i8,
                    dy: data[2] as i8,
                })
            }
            RawPacketType::TrackpadZoom => {
                if data.len() < 2 { return None; }
                Some(RawPacket::TrackpadZoom { direction: data[1] as i8 })
            }
            RawPacketType::TrackpadSwipe => {
                if data.len() < 2 { return None; }
                Some(RawPacket::TrackpadSwipe { direction: data[1] })
            }
            RawPacketType::Keyboard => {
                if data.len() < 2 { return None; }
                Some(RawPacket::Keyboard { key_code: data[1] })
            }
            RawPacketType::Gyro => {
                if data.len() < 8 { return None; }
                let pitch = i16::from_be_bytes([data[1], data[2]]);
                let yaw = i16::from_be_bytes([data[3], data[4]]);
                let roll = i16::from_be_bytes([data[5], data[6]]);
                let inverted = data[7] != 0;
                Some(RawPacket::Gyro { pitch, yaw, roll, inverted })
            }
            RawPacketType::Clipboard => {
                Some(RawPacket::Clipboard(data[1..].to_vec()))
            }
        };
    }

    // No type prefix — treat as a raw 8-byte controller packet
    if data.len() >= 7 {
        Some(RawPacket::Controller {
            left_x: data[0],
            left_y: data[1],
            right_x: data[2],
            right_y: data[3],
            trigger_l: data[4],
            trigger_r: data[5],
            buttons: data[6],
        })
    } else {
        None
    }
}

// --- Typed Payloads ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakePayload {
    pub device_name: String,
    pub device_id: uuid::Uuid,
    pub protocol_version: u32,
    pub capabilities: DeviceCapabilities,
    pub session_token: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub has_gyroscope: bool,
    pub has_accelerometer: bool,
    pub has_touchpad: bool,
    pub has_rumble: bool,
    pub max_battery_level: u8,
    pub supported_transports: Vec<TransportType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransportType {
    Udp = 0,
    Bluetooth = 1,
    Usb = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPayload {
    pub buttons: u32,
    pub left_stick: StickAxis,
    pub right_stick: StickAxis,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub dpad: DpadState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StickAxis {
    pub x: i16,
    pub y: i16,
}

impl StickAxis {
    pub const CENTER: Self = Self { x: 0, y: 0 };
    pub const DEADZONE: i16 = 800;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DpadState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputBatchPayload {
    pub inputs: Vec<InputPayload>,
    pub base_timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryPayload {
    pub level: u8,
    pub charging: bool,
    pub voltage_mv: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyroscopePayload {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub timestamp_delta_ms: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchpadPayload {
    pub touch_id: u8,
    pub x: u16,
    pub y: u16,
    pub pressure: u8,
    pub touching: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSwitchPayload {
    pub profile_id: uuid::Uuid,
    pub profile_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoPayload {
    pub name: String,
    pub firmware_version: String,
    pub model: String,
    pub serial: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: u16,
    pub message: String,
}

// Button bitflags
pub mod buttons {
    pub const A: u32 = 1 << 0;
    pub const B: u32 = 1 << 1;
    pub const X: u32 = 1 << 2;
    pub const Y: u32 = 1 << 3;
    pub const LB: u32 = 1 << 4;
    pub const RB: u32 = 1 << 5;
    pub const BACK: u32 = 1 << 6;
    pub const START: u32 = 1 << 7;
    pub const GUIDE: u32 = 1 << 8;
    pub const LEFT_STICK: u32 = 1 << 9;
    pub const RIGHT_STICK: u32 = 1 << 10;
    pub const DPAD_UP: u32 = 1 << 11;
    pub const DPAD_DOWN: u32 = 1 << 12;
    pub const DPAD_LEFT: u32 = 1 << 13;
    pub const DPAD_RIGHT: u32 = 1 << 14;
    pub const TOUCHPAD: u32 = 1 << 15;
}
