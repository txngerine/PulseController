use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }

    pub fn as_u32(&self) -> u32 {
        ((self.major as u32) << 16) | (self.minor as u32)
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
