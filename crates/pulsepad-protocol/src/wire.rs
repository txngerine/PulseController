use crate::error::{ProtocolError, Result};
use crate::packet::{Packet, PacketType};
use bytes::{BufMut, BytesMut};

pub const MAGIC: [u8; 2] = [0x50, 0x50]; // 'PP'
pub const HEADER_SIZE: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamId {
    Control = 0x00,
    Gamepad = 0x01,
    Keyboard = 0x02,
    Mouse = 0x03,
    Gyroscope = 0x04,
    Haptics = 0x05,
    TouchMacro = 0x06,
    Audio = 0x07,
    Heartbeat = 0x08,
    ClockSync = 0x09,
    FileSync = 0x0A,
}

impl StreamId {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x00 => Some(Self::Control),
            0x01 => Some(Self::Gamepad),
            0x02 => Some(Self::Keyboard),
            0x03 => Some(Self::Mouse),
            0x04 => Some(Self::Gyroscope),
            0x05 => Some(Self::Haptics),
            0x06 => Some(Self::TouchMacro),
            0x07 => Some(Self::Audio),
            0x08 => Some(Self::Heartbeat),
            0x09 => Some(Self::ClockSync),
            0x0A => Some(Self::FileSync),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PacketFlags: u8 {
        const ACK        = 0b0000_0001;
        const RELIABLE   = 0b0000_0010;
        const SYN        = 0b0000_0100;
        const FIN        = 0b0000_1000;
        const COMPRESSED = 0b0001_0000;
        const AUTH       = 0b0010_0000;
    }
}

#[derive(Debug, Clone)]
pub struct WireFrame {
    pub version: u8,
    pub flags: PacketFlags,
    pub stream_id: StreamId,
    pub sequence: u32,
    pub payload: Vec<u8>,
}

impl WireFrame {
    pub fn new(stream_id: StreamId, flags: PacketFlags, sequence: u32, payload: Vec<u8>) -> Self {
        Self {
            version: 1,
            flags,
            stream_id,
            sequence,
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(HEADER_SIZE + self.payload.len());
        buf.put_slice(&MAGIC);
        buf.put_u8(self.version);
        buf.put_u8(self.flags.bits());
        buf.put_u16_le(self.stream_id as u16);
        buf.put_u32_le(self.sequence);
        buf.put_u32_le(self.payload.len() as u32);
        buf.put_slice(&self.payload);
        buf.to_vec()
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_SIZE {
            return Err(ProtocolError::PacketTooSmall(data.len()));
        }
        if data[0] != MAGIC[0] || data[1] != MAGIC[1] {
            return Err(ProtocolError::InvalidPacketType(data[0]));
        }
        let version = data[2];
        let flags = PacketFlags::from_bits_truncate(data[3]);
        let stream_id = u16::from_le_bytes([data[4], data[5]]);
        let sid = StreamId::from_u16(stream_id)
            .ok_or(ProtocolError::InvalidPacketType(stream_id as u8))?;
        let sequence = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
        let payload_len = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;

        let total = HEADER_SIZE + payload_len;
        if data.len() < total {
            return Err(ProtocolError::PacketTooSmall(data.len()));
        }
        let payload = data[HEADER_SIZE..total].to_vec();

        Ok(Self {
            version,
            flags,
            stream_id: sid,
            sequence,
            payload,
        })
    }
}

/// Convert a legacy typed Packet into a WireFrame on the Control stream.
pub fn packet_to_frame(packet: &Packet, sequence: u32) -> Result<WireFrame> {
    let inner = bincode::serialize(packet)?;
    let ptype = packet.packet_type().unwrap_or(PacketType::Reserved);
    let flags = if matches!(ptype, PacketType::Handshake | PacketType::HandshakeAck) {
        PacketFlags::SYN | PacketFlags::RELIABLE
    } else {
        PacketFlags::empty()
    };
    Ok(WireFrame::new(StreamId::Control, flags, sequence, inner))
}

/// Convert a WireFrame back into a typed Packet (for Control stream).
pub fn frame_to_packet(frame: &WireFrame) -> Result<Packet> {
    let packet: Packet = bincode::deserialize(&frame.payload)?;
    Ok(packet)
}
