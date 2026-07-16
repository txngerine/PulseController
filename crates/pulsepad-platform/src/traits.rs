use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub mouse_sensitivity: f32,
    pub mouse_acceleration: bool,
    pub keyboard_repeat_rate: u32,
    pub keyboard_repeat_delay: u32,
    pub media_key_emulation: bool,
    pub controller_vibration: bool,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            mouse_acceleration: false,
            keyboard_repeat_rate: 30,
            keyboard_repeat_delay: 500,
            media_key_emulation: true,
            controller_vibration: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendEvent {
    ControllerConnected { id: u32, name: String },
    ControllerDisconnected { id: u32 },
    VibrationEvent { id: u32, left_motor: u8, right_motor: u8 },
    LedEvent { id: u32, r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaKey {
    PlayPause,
    NextTrack,
    PreviousTrack,
    Stop,
    VolumeUp,
    VolumeDown,
    Mute,
    Mail,
    Calculator,
    Explorer,
}

impl fmt::Display for MediaKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayPause => write!(f, "PlayPause"),
            Self::NextTrack => write!(f, "NextTrack"),
            Self::PreviousTrack => write!(f, "PreviousTrack"),
            Self::Stop => write!(f, "Stop"),
            Self::VolumeUp => write!(f, "VolumeUp"),
            Self::VolumeDown => write!(f, "VolumeDown"),
            Self::Mute => write!(f, "Mute"),
            Self::Mail => write!(f, "Mail"),
            Self::Calculator => write!(f, "Calculator"),
            Self::Explorer => write!(f, "Explorer"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerState {
    pub id: u32,
    pub name: String,
    pub connected: bool,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub left_stick_x: i16,
    pub left_stick_y: i16,
    pub right_stick_x: i16,
    pub right_stick_y: i16,
    pub buttons: u32,
    pub dpad: u8,
}

#[async_trait]
pub trait InputBackend: Send + Sync + fmt::Debug {
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;

    async fn initialize(&mut self, config: BackendConfig) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;

    async fn send_controller_state(&self, state: &ControllerState) -> Result<()>;
    async fn send_key_press(&self, key: u32, down: bool) -> Result<()>;
    async fn send_mouse_move(&self, dx: i32, dy: i32) -> Result<()>;
    async fn send_mouse_button(&self, button: MouseButton, down: bool) -> Result<()>;
    async fn send_mouse_scroll(&self, x: i32, y: i32) -> Result<()>;
    async fn send_media_key(&self, key: MediaKey, down: bool) -> Result<()>;

    fn supported_features(&self) -> Vec<&str> {
        vec!["keyboard", "mouse", "media"]
    }
}
