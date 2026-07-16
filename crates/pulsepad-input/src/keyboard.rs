use std::collections::HashMap;
use tracing::debug;

use crate::controller::ButtonMapping;

#[derive(Debug, Clone)]
pub struct KeyboardMapper {
    mappings: HashMap<u32, Vec<ButtonMapping>>,
    key_states: HashMap<u32, bool>,
}

impl KeyboardMapper {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            key_states: HashMap::new(),
        }
    }

    pub fn load_mappings(&mut self, mappings: Vec<ButtonMapping>) {
        self.mappings.clear();
        for mapping in mappings {
            self.mappings
                .entry(mapping.button)
                .or_default()
                .push(mapping);
        }
        debug!("loaded {} keyboard mappings", self.mappings.len());
    }

    pub fn process_button(&mut self, button: u32, pressed: bool) -> Vec<(u32, bool)> {
        let mut actions = Vec::new();

        if let Some(mappings) = self.mappings.get(&button) {
            for mapping in mappings {
                if let crate::controller::InputAction::KeyPress { key } = mapping.action {
                    let was_pressed = self.key_states.get(&key).copied().unwrap_or(false);

                    if pressed != was_pressed {
                        self.key_states.insert(key, pressed);
                        actions.push((key, pressed));
                    }
                }
            }
        }

        actions
    }

    pub fn release_all(&mut self) -> Vec<(u32, bool)> {
        let keys_to_release: Vec<u32> = self
            .key_states
            .iter()
            .filter(|(_, &pressed)| pressed)
            .map(|(&key, _)| key)
            .collect();
        for key in &keys_to_release {
            self.key_states.insert(*key, false);
        }
        keys_to_release.into_iter().map(|k| (k, false)).collect()
    }

    pub fn active_keys(&self) -> Vec<u32> {
        self.key_states
            .iter()
            .filter(|(_, &pressed)| pressed)
            .map(|(&key, _)| key)
            .collect()
    }
}

impl Default for KeyboardMapper {
    fn default() -> Self {
        Self::new()
    }
}
