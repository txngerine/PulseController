use async_trait::async_trait;
use tracing::{debug, info};

use crate::error::{PlatformError, Result};
use crate::traits::{BackendConfig, InputBackend, ControllerState, MouseButton, MediaKey};

#[derive(Debug)]
pub struct LinuxBackend {
    config: BackendConfig,
    initialized: bool,
}

impl LinuxBackend {
    pub fn new() -> Self {
        Self {
            config: BackendConfig::default(),
            initialized: false,
        }
    }
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InputBackend for LinuxBackend {
    fn name(&self) -> &str {
        "Linux"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn initialize(&mut self, config: BackendConfig) -> Result<()> {
        self.config = config;
        self.initialized = true;
        info!("Linux backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        info!("Linux backend shut down");
        Ok(())
    }

    async fn send_controller_state(&self, state: &ControllerState) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("controller state: id={}, left_stick=({},{}) - Linux stub, no-op",
            state.id, state.left_stick_x, state.left_stick_y);
        Ok(())
    }

    async fn send_key_press(&self, key: u32, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("key press: key={}, down={} - Linux stub, no-op", key, down);
        Ok(())
    }

    async fn send_mouse_move(&self, dx: i32, dy: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("mouse move: dx={}, dy={} - Linux stub, no-op", dx, dy);
        Ok(())
    }

    async fn send_mouse_button(&self, button: MouseButton, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("mouse button: {:?}, down={} - Linux stub, no-op", button, down);
        Ok(())
    }

    async fn send_mouse_scroll(&self, x: i32, y: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("mouse scroll: x={}, y={} - Linux stub, no-op", x, y);
        Ok(())
    }

    async fn send_media_key(&self, key: MediaKey, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("media key: {:?}, down={} - Linux stub, no-op", key, down);
        Ok(())
    }

    fn supported_features(&self) -> Vec<&str> {
        vec!["keyboard", "mouse", "media"]
    }
}
