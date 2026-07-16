use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AppState {
    pub connected: bool,
    pub device_name: Option<String>,
    pub latency_ms: f64,
    pub packet_loss: f64,
    pub battery_level: Option<u8>,
    pub active_profile: Option<String>,
    pub transport: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: false,
            device_name: None,
            latency_ms: 0.0,
            packet_loss: 0.0,
            battery_level: None,
            active_profile: None,
            transport: "UDP".to_string(),
        }
    }
}
