use pulsepad_protocol::packet::TransportType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub is_default: bool,
    pub controller_mappings: Vec<ControllerMapping>,
    pub keyboard_mappings: Vec<KeyboardMapping>,
    pub mouse_mappings: Vec<MouseMapping>,
    pub media_mappings: Vec<MediaMapping>,
    pub sensitivity: SensitivitySettings,
    pub deadzone: DeadzoneSettings,
    pub macros: Vec<Macro>,
    pub lighting: Option<LightingSettings>,
    pub transport_preferences: TransportPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerMapping {
    pub button: u32,
    pub action: MappingAction,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingAction {
    KeyPress { key: String },
    MouseButton { button: String },
    MouseMove { sensitivity: f32 },
    Scroll { sensitivity: f32 },
    MediaKey { key: String },
    Macro { macro_id: Uuid },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardMapping {
    pub source_key: String,
    pub target_key: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMapping {
    pub source_button: String,
    pub target_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMapping {
    pub source_button: String,
    pub target_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivitySettings {
    pub mouse_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub stick_sensitivity: f32,
    pub trigger_sensitivity: f32,
}

impl Default for SensitivitySettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            scroll_sensitivity: 1.0,
            stick_sensitivity: 1.0,
            trigger_sensitivity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadzoneSettings {
    pub left_stick: i16,
    pub right_stick: i16,
    pub left_trigger: u8,
    pub right_trigger: u8,
}

impl Default for DeadzoneSettings {
    fn default() -> Self {
        Self {
            left_stick: 800,
            right_stick: 800,
            left_trigger: 20,
            right_trigger: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub id: Uuid,
    pub name: String,
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroStep {
    pub action: String,
    pub delay_ms: u64,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingSettings {
    pub enabled: bool,
    pub color: Color,
    pub brightness: u8,
    pub mode: LightingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum LightingMode {
    Solid = 0,
    Pulse = 1,
    Rainbow = 2,
    Reactive = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportPreferences {
    pub preferred: TransportType,
    pub fallback: Vec<TransportType>,
    pub auto_switch: bool,
}

impl Default for TransportPreferences {
    fn default() -> Self {
        Self {
            preferred: TransportType::Udp,
            fallback: vec![TransportType::Bluetooth],
            auto_switch: true,
        }
    }
}

impl Profile {
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            version: 1,
            created_at: now.clone(),
            updated_at: now,
            is_default: false,
            controller_mappings: Vec::new(),
            keyboard_mappings: Vec::new(),
            mouse_mappings: Vec::new(),
            media_mappings: Vec::new(),
            sensitivity: SensitivitySettings::default(),
            deadzone: DeadzoneSettings::default(),
            macros: Vec::new(),
            lighting: None,
            transport_preferences: TransportPreferences::default(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("profile name cannot be empty".to_string());
        }

        if self.sensitivity.mouse_sensitivity <= 0.0 {
            return Err("mouse sensitivity must be positive".to_string());
        }

        Ok(())
    }
}
