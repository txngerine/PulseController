use async_trait::async_trait;
use tracing::{debug, error, info, warn};

use crate::error::{PlatformError, Result};
use crate::traits::{BackendConfig, InputBackend, ControllerState, MouseButton, MediaKey};

#[derive(Debug)]
pub struct WindowsBackend {
    config: BackendConfig,
    initialized: bool,
}

impl WindowsBackend {
    pub fn new() -> Self {
        Self {
            config: BackendConfig::default(),
            initialized: false,
        }
    }
}

impl Default for WindowsBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InputBackend for WindowsBackend {
    fn name(&self) -> &str {
        "Windows"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn initialize(&mut self, config: BackendConfig) -> Result<()> {
        self.config = config;
        self.initialized = true;
        info!("Windows backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        info!("Windows backend shut down");
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

        #[cfg(target_os = "windows")]
        {
            // TODO: Implement XInput/virtual Xbox controller via Windows API
            // This would use the windows crate to send controller input
            // For now, log the state for debugging
            warn!("Windows controller backend not yet fully implemented");
        }

        #[cfg(not(target_os = "windows"))]
        {
            warn!("Windows backend called on non-Windows platform");
        }

        Ok(())
    }

    async fn send_key_press(&self, key: u32, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("key press: key={}, down={}", key, down);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            let vk = key as u16;
            let flags = if down { KEYDOWN } else { KEYUP };

            unsafe {
                let inputs = [INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(vk),
                            dwFlags: flags,
                            ..Default::default()
                        },
                    },
                }];

                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
        }

        Ok(())
    }

    async fn send_mouse_move(&self, dx: i32, dy: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("mouse move: dx={}, dy={}", dx, dy);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            unsafe {
                let inputs = [INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx,
                            dy,
                            dwFlags: MOUSEEVENTF_MOVE,
                            ..Default::default()
                        },
                    },
                }];

                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
        }

        Ok(())
    }

    async fn send_mouse_button(&self, button: MouseButton, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("mouse button: {:?}, down={}", button, down);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            let (flags, mouse_data) = match button {
                MouseButton::Left => {
                    (if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP }, 0)
                }
                MouseButton::Right => {
                    (if down { MOUSEEVENTF_RIGHTDOWN } else { MOUSEEVENTF_RIGHTUP }, 0)
                }
                MouseButton::Middle => {
                    (if down { MOUSEEVENTF_MIDDLEDOWN } else { MOUSEEVENTF_MIDDLEUP }, 0)
                }
                MouseButton::X1 => {
                    (if down { MOUSEEVENTF_XDOWN } else { MOUSEEVENTF_XUP }, 1)
                }
                MouseButton::X2 => {
                    (if down { MOUSEEVENTF_XDOWN } else { MOUSEEVENTF_XUP }, 2)
                }
            };

            unsafe {
                let inputs = [INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dwFlags: flags,
                            mouseData: mouse_data,
                            ..Default::default()
                        },
                    },
                }];

                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
        }

        Ok(())
    }

    async fn send_mouse_scroll(&self, x: i32, y: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("mouse scroll: x={}, y={}", x, y);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            if y != 0 {
                unsafe {
                    let inputs = [INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dwFlags: MOUSEEVENTF_WHEEL,
                                mouseData: y * 120,
                                ..Default::default()
                            },
                        },
                    }];
                    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                }
            }

            if x != 0 {
                unsafe {
                    let inputs = [INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dwFlags: MOUSEEVENTF_HWHEEL,
                                mouseData: x * 120,
                                ..Default::default()
                            },
                        },
                    }];
                    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                }
            }
        }

        Ok(())
    }

    async fn send_media_key(&self, key: MediaKey, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }

        debug!("media key: {:?}, down={}", key, down);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;

            let vk = match key {
                MediaKey::PlayPause => 0xB3,
                MediaKey::NextTrack => 0xB0,
                MediaKey::PreviousTrack => 0xB1,
                MediaKey::Stop => 0xB2,
                MediaKey::VolumeUp => 0xAF,
                MediaKey::VolumeDown => 0xAE,
                MediaKey::Mute => 0xAD,
                MediaKey::Mail => 0xAC,
                MediaKey::Calculator => 0xB2,
                MediaKey::Explorer => 0xB6,
            };

            let flags = if down { KEYDOWN } else { KEYUP };

            unsafe {
                let inputs = [INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(vk),
                            dwFlags: flags | KEYEVENTF_EXTENDEDKEY,
                            ..Default::default()
                        },
                    },
                }];

                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
        }

        Ok(())
    }

    fn supported_features(&self) -> Vec<&str> {
        vec!["keyboard", "mouse", "media", "controller"]
    }
}
