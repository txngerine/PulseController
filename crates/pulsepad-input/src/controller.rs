use serde::{Deserialize, Serialize};

use pulsepad_protocol::packet::{InputPayload, StickAxis};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerInput {
    pub buttons: u32,
    pub left_stick: StickAxis,
    pub right_stick: StickAxis,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
}

impl From<InputPayload> for ControllerInput {
    fn from(payload: InputPayload) -> Self {
        Self {
            buttons: payload.buttons,
            left_stick: payload.left_stick,
            right_stick: payload.right_stick,
            left_trigger: payload.left_trigger,
            right_trigger: payload.right_trigger,
            dpad_up: payload.dpad.up,
            dpad_down: payload.dpad.down,
            dpad_left: payload.dpad.left,
            dpad_right: payload.dpad.right,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapping {
    pub button: u32,
    pub action: InputAction,
    pub modifiers: Vec<KeyModifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputAction {
    KeyPress { key: u32 },
    MouseButton { button: u8 },
    MouseMove { dx: i32, dy: i32 },
    MouseScroll { x: i32, y: i32 },
    MediaKey { key: u8 },
    Macro { macro_id: String },
    ProfileSwitch { profile_id: String },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyModifier {
    Shift,
    Control,
    Alt,
    Super,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickMapping {
    pub deadzone: i16,
    pub sensitivity: f32,
    pub curve: StickCurve,
    pub action_x: InputAction,
    pub action_y: InputAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StickCurve {
    Linear,
    Exponential,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerMapping {
    pub deadzone: u8,
    pub threshold: u8,
    pub action: InputAction,
}

impl ControllerInput {
    pub fn has_button(&self, button: u32) -> bool {
        self.buttons & button != 0
    }

    pub fn left_stick_processed(&self, deadzone: i16) -> (f32, f32) {
        apply_deadzone(self.left_stick.x, self.left_stick.y, deadzone)
    }

    pub fn right_stick_processed(&self, deadzone: i16) -> (f32, f32) {
        apply_deadzone(self.right_stick.x, self.right_stick.y, deadzone)
    }
}

fn apply_deadzone(x: i16, y: i16, deadzone: i16) -> (f32, f32) {
    let magnitude = ((x as f32).powi(2) + (y as f32).powi(2)).sqrt();
    let deadzone_f = deadzone as f32;

    if magnitude < deadzone_f {
        return (0.0, 0.0);
    }

    let normalized = (magnitude - deadzone_f) / (i16::MAX as f32 - deadzone_f);
    let scale = normalized / magnitude;

    (x as f32 * scale, y as f32 * scale)
}
