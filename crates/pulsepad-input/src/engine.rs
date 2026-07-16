use tracing::{debug, info, warn};

use crate::error::Result;
use crate::controller::{ControllerInput, ButtonMapping, StickMapping, TriggerMapping};
use crate::keyboard::KeyboardMapper;
use crate::mouse::MouseMapper;
use crate::media::MediaMapper;
use pulsepad_platform::traits::{InputBackend, MouseButton, MediaKey};

#[derive(Debug, Clone)]
pub struct InputEngine {
    keyboard: KeyboardMapper,
    mouse: MouseMapper,
    media: MediaMapper,
    button_mappings: Vec<ButtonMapping>,
    stick_mappings: Vec<StickMapping>,
    trigger_mappings: Vec<TriggerMapping>,
    mouse_button_mappings: Vec<ButtonMapping>,
}

impl InputEngine {
    pub fn new() -> Self {
        Self {
            keyboard: KeyboardMapper::new(),
            mouse: MouseMapper::new(),
            media: MediaMapper::new(),
            button_mappings: Vec::new(),
            stick_mappings: Vec::new(),
            trigger_mappings: Vec::new(),
            mouse_button_mappings: Vec::new(),
        }
    }

    pub fn load_profile(
        &mut self,
        button_mappings: Vec<ButtonMapping>,
        stick_mappings: Vec<StickMapping>,
        trigger_mappings: Vec<TriggerMapping>,
        mouse_button_mappings: Vec<ButtonMapping>,
        sensitivity: f32,
        _deadzone: i16,
    ) {
        self.keyboard.load_mappings(button_mappings.clone());
        self.media.load_mappings(button_mappings.clone());
        self.mouse.set_sensitivity(sensitivity);

        self.button_mappings = button_mappings;
        self.stick_mappings = stick_mappings;
        self.trigger_mappings = trigger_mappings;
        self.mouse_button_mappings = mouse_button_mappings;

        info!(
            "profile loaded: {} button, {} stick, {} trigger, {} mouse button mappings",
            self.button_mappings.len(),
            self.stick_mappings.len(),
            self.trigger_mappings.len(),
            self.mouse_button_mappings.len()
        );
    }

    pub async fn process_input(
        &mut self,
        input: &ControllerInput,
        backend: &dyn InputBackend,
    ) -> Result<()> {
        self.process_buttons(input, backend).await?;
        self.process_sticks(input, backend).await?;
        self.process_triggers(input, backend).await?;
        Ok(())
    }

    async fn process_buttons(
        &mut self,
        input: &ControllerInput,
        backend: &dyn InputBackend,
    ) -> Result<()> {
        // Keyboard mappings
        let mut all_keyboard_actions = Vec::new();

        for mapping in &self.button_mappings {
            if input.has_button(mapping.button) || mapping.button == 0 {
                continue;
            }
            let actions = self.keyboard.process_button(mapping.button, false);
            all_keyboard_actions.extend(actions);
        }

        for mapping in &self.button_mappings {
            if !input.has_button(mapping.button) {
                continue;
            }
            let actions = self.keyboard.process_button(mapping.button, true);
            all_keyboard_actions.extend(actions);
        }

        for (key, pressed) in all_keyboard_actions {
            if let Err(e) = backend.send_key_press(key, pressed).await {
                warn!("failed to send key press: {}", e);
            }
        }

        // Media key mappings
        for mapping in &self.button_mappings {
            if let Some((key, pressed)) = self.media.process_button(mapping.button, input.has_button(mapping.button)) {
                let media_key = match key {
                    0 => MediaKey::PlayPause,
                    1 => MediaKey::NextTrack,
                    2 => MediaKey::PreviousTrack,
                    3 => MediaKey::Stop,
                    4 => MediaKey::VolumeUp,
                    5 => MediaKey::VolumeDown,
                    6 => MediaKey::Mute,
                    _ => continue,
                };
                if let Err(e) = backend.send_media_key(media_key, pressed).await {
                    warn!("failed to send media key: {}", e);
                }
            }
        }

        // Mouse button mappings
        for mapping in &self.mouse_button_mappings {
            if input.has_button(mapping.button) {
                if let crate::controller::InputAction::MouseButton { button } = mapping.action {
                    let mouse_button = match button {
                        0 => MouseButton::Left,
                        1 => MouseButton::Right,
                        2 => MouseButton::Middle,
                        3 => MouseButton::X1,
                        4 => MouseButton::X2,
                        _ => continue,
                    };
                    if let Err(e) = backend.send_mouse_button(mouse_button, true).await {
                        warn!("failed to send mouse button: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_sticks(
        &mut self,
        input: &ControllerInput,
        backend: &dyn InputBackend,
    ) -> Result<()> {
        // Left stick to mouse movement
        let (left_x, left_y) = input.left_stick_processed(800);
        if left_x.abs() > 0.01 || left_y.abs() > 0.01 {
            if let Some(mapping) = self.stick_mappings.first() {
                let (dx, dy) = self.mouse.process_stick_movement(left_x, left_y, mapping);
                if dx != 0 || dy != 0 {
                    if let Err(e) = backend.send_mouse_move(dx, dy).await {
                        warn!("failed to send mouse move: {}", e);
                    }
                }
            }
        }

        // Right stick to scroll
        let (right_x, right_y) = input.right_stick_processed(800);
        if right_x.abs() > 0.01 || right_y.abs() > 0.01 {
            let (scroll_x, scroll_y) = self.mouse.process_scroll(right_x * 10.0, right_y * 10.0);
            if scroll_x != 0 || scroll_y != 0 {
                if let Err(e) = backend.send_mouse_scroll(scroll_x, scroll_y).await {
                    warn!("failed to send mouse scroll: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn process_triggers(
        &mut self,
        input: &ControllerInput,
        _backend: &dyn InputBackend,
    ) -> Result<()> {
        // Triggers can be mapped to various actions
        // For now, log them for debugging
        if input.left_trigger > 10 {
            debug!("left trigger: {}", input.left_trigger);
        }
        if input.right_trigger > 10 {
            debug!("right trigger: {}", input.right_trigger);
        }

        Ok(())
    }

    pub fn release_all(&mut self) -> Vec<(u32, bool)> {
        self.keyboard.release_all()
    }

    pub fn active_keys(&self) -> Vec<u32> {
        self.keyboard.active_keys()
    }
}

impl Default for InputEngine {
    fn default() -> Self {
        Self::new()
    }
}
