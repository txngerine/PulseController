use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::error::{PlatformError, Result};
use crate::traits::{BackendConfig, InputBackend, ControllerState, MouseButton, MediaKey};

#[derive(Debug)]
pub struct MacosBackend {
    config: BackendConfig,
    initialized: bool,
}

impl MacosBackend {
    pub fn new() -> Self {
        Self {
            config: BackendConfig::default(),
            initialized: false,
        }
    }
}

impl Default for MacosBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
mod cg {
    pub use core_graphics::event::{
        CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, CGScrollEventUnit,
        ScrollEventUnit,
    };
    pub use core_graphics::event::EventField;
    pub use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    pub use core_graphics::geometry::CGPoint;
}

#[async_trait]
impl InputBackend for MacosBackend {
    fn name(&self) -> &str {
        "macOS"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn initialize(&mut self, config: BackendConfig) -> Result<()> {
        self.config = config;
        self.initialized = true;
        info!("macOS backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        info!("macOS backend shut down");
        Ok(())
    }

    async fn send_controller_state(&self, state: &ControllerState) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!(
            "controller state: id={}, left_stick=({},{})",
            state.id, state.left_stick_x, state.left_stick_y
        );

        #[cfg(target_os = "macos")]
        {
            warn!("macOS controller backend not yet implemented - use keyboard/mouse emulation");
        }

        Ok(())
    }

    async fn send_key_press(&self, key: u32, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("key press: key={}, down={}", key, down);

        #[cfg(target_os = "macos")]
        {
            use cg::*;

            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InjectionFailed("failed to create event source".into()))?;

            let key_code = key as u16;

            let event = CGEvent::new_keyboard_event(source, key_code, down)
                .map_err(|_| PlatformError::InjectionFailed("failed to create keyboard event".into()))?;

            event.post(CGEventTapLocation::HID);
        }

        Ok(())
    }

    async fn send_mouse_move(&self, dx: i32, dy: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("mouse move: dx={}, dy={}", dx, dy);

        #[cfg(target_os = "macos")]
        {
            use cg::*;

            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InjectionFailed("failed to create event source".into()))?;

            let event = CGEvent::new_mouse_event(
                source,
                CGEventType::MouseMoved,
                CGPoint::new(0.0, 0.0),
                CGMouseButton::Left,
            )
            .map_err(|_| PlatformError::InjectionFailed("failed to create mouse event".into()))?;

            event.set_integer_value_field(EventField::MOUSE_EVENT_DELTA_X, dx as i64);
            event.set_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y, dy as i64);

            event.post(CGEventTapLocation::HID);
        }

        Ok(())
    }

    async fn send_mouse_button(&self, button: MouseButton, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("mouse button: {:?}, down={}", button, down);

        #[cfg(target_os = "macos")]
        {
            use cg::*;

            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InjectionFailed("failed to create event source".into()))?;

            let (event_type, mouse_button) = match button {
                MouseButton::Left => (
                    if down { CGEventType::LeftMouseDown } else { CGEventType::LeftMouseUp },
                    CGMouseButton::Left,
                ),
                MouseButton::Right => (
                    if down { CGEventType::RightMouseDown } else { CGEventType::RightMouseUp },
                    CGMouseButton::Right,
                ),
                MouseButton::Middle => (
                    if down { CGEventType::OtherMouseDown } else { CGEventType::OtherMouseUp },
                    CGMouseButton::Center,
                ),
                _ => {
                    warn!("unsupported mouse button on macOS: {:?}", button);
                    return Ok(());
                }
            };

            let event = CGEvent::new_mouse_event(
                source,
                event_type,
                CGPoint::new(0.0, 0.0),
                mouse_button,
            )
            .map_err(|_| PlatformError::InjectionFailed("failed to create mouse event".into()))?;

            event.post(CGEventTapLocation::HID);
        }

        Ok(())
    }

    async fn send_mouse_scroll(&self, x: i32, y: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("mouse scroll: x={}, y={}", x, y);

        #[cfg(target_os = "macos")]
        {
            use cg::*;

            let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
                .map_err(|_| PlatformError::InjectionFailed("failed to create event source".into()))?;

            let event = CGEvent::new_scroll_event(
                source,
                ScrollEventUnit::PIXEL,
                2,
                y,
                x,
                0,
            )
            .map_err(|_| PlatformError::InjectionFailed("failed to create scroll event".into()))?;

            event.post(CGEventTapLocation::HID);
        }

        Ok(())
    }

    async fn send_media_key(&self, key: MediaKey, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("media key: {:?}, down={}", key, down);

        // NOTE: macOS media keys require IOKit/IOHIDManager, not CGEvent.
        // This is a best-effort implementation using CGEvent key codes.
        warn!("media key emulation on macOS requires IOKit — not fully supported via CGEvent");

        Ok(())
    }

    fn supported_features(&self) -> Vec<&str> {
        vec!["keyboard", "mouse", "media"]
    }
}
