use std::collections::HashMap;
use tracing::debug;

use crate::controller::ButtonMapping;

#[derive(Debug, Clone)]
pub struct MediaMapper {
    mappings: HashMap<u32, u8>,
}

impl MediaMapper {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn load_mappings(&mut self, mappings: Vec<ButtonMapping>) {
        self.mappings.clear();
        for mapping in mappings {
            if let crate::controller::InputAction::MediaKey { key } = mapping.action {
                self.mappings.insert(mapping.button, key);
            }
        }
        debug!("loaded {} media key mappings", self.mappings.len());
    }

    pub fn get_media_key(&self, button: u32) -> Option<u8> {
        self.mappings.get(&button).copied()
    }

    pub fn process_button(&self, button: u32, pressed: bool) -> Option<(u8, bool)> {
        self.get_media_key(button).map(|key| (key, pressed))
    }
}

impl Default for MediaMapper {
    fn default() -> Self {
        Self::new()
    }
}
