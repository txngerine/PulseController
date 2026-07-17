use async_trait::async_trait;
use evdev::{uinput::VirtualDevice, uinput::VirtualDeviceBuilder, AttributeSet, InputEvent, EventType, Key, RelativeAxisType};
use std::collections::HashSet;
use std::sync::Mutex;
use tracing::{debug, error, info};

use crate::error::{PlatformError, Result};
use crate::traits::{BackendConfig, InputBackend, ControllerState, MouseButton, MediaKey};

const EVDEV_KEYBOARD_NAME: &str = "PulsePad Virtual Keyboard";
const EVDEV_MOUSE_NAME: &str = "PulsePad Virtual Mouse";

fn keycode_to_evdev(key: u32) -> Option<Key> {
    // Map from platform-independent key codes to evdev KEY_* codes
    Some(match key {
        0 => Key::KEY_RESERVED,
        1 => Key::KEY_ESC,
        2 => Key::KEY_1,
        3 => Key::KEY_2,
        4 => Key::KEY_3,
        5 => Key::KEY_4,
        6 => Key::KEY_5,
        7 => Key::KEY_6,
        8 => Key::KEY_7,
        9 => Key::KEY_8,
        10 => Key::KEY_9,
        11 => Key::KEY_0,
        12 => Key::KEY_MINUS,
        13 => Key::KEY_EQUAL,
        14 => Key::KEY_BACKSPACE,
        15 => Key::KEY_TAB,
        16 => Key::KEY_Q,
        17 => Key::KEY_W,
        18 => Key::KEY_E,
        19 => Key::KEY_R,
        20 => Key::KEY_T,
        21 => Key::KEY_Y,
        22 => Key::KEY_U,
        23 => Key::KEY_I,
        24 => Key::KEY_O,
        25 => Key::KEY_P,
        26 => Key::KEY_LEFTBRACE,
        27 => Key::KEY_RIGHTBRACE,
        28 => Key::KEY_ENTER,
        29 => Key::KEY_LEFTCTRL,
        30 => Key::KEY_A,
        31 => Key::KEY_S,
        32 => Key::KEY_D,
        33 => Key::KEY_F,
        34 => Key::KEY_G,
        35 => Key::KEY_H,
        36 => Key::KEY_J,
        37 => Key::KEY_K,
        38 => Key::KEY_L,
        39 => Key::KEY_SEMICOLON,
        40 => Key::KEY_APOSTROPHE,
        41 => Key::KEY_GRAVE,
        42 => Key::KEY_LEFTSHIFT,
        43 => Key::KEY_BACKSLASH,
        44 => Key::KEY_Z,
        45 => Key::KEY_X,
        46 => Key::KEY_C,
        47 => Key::KEY_V,
        48 => Key::KEY_B,
        49 => Key::KEY_N,
        50 => Key::KEY_M,
        51 => Key::KEY_COMMA,
        52 => Key::KEY_DOT,
        53 => Key::KEY_SLASH,
        54 => Key::KEY_RIGHTSHIFT,
        55 => Key::KEY_KPASTERISK,
        56 => Key::KEY_LEFTALT,
        57 => Key::KEY_SPACE,
        58 => Key::KEY_CAPSLOCK,
        59 => Key::KEY_F1,
        60 => Key::KEY_F2,
        61 => Key::KEY_F3,
        62 => Key::KEY_F4,
        63 => Key::KEY_F5,
        64 => Key::KEY_F6,
        65 => Key::KEY_F7,
        66 => Key::KEY_F8,
        67 => Key::KEY_F9,
        68 => Key::KEY_F10,
        69 => Key::KEY_NUMLOCK,
        70 => Key::KEY_SCROLLLOCK,
        71 => Key::KEY_KP7,
        72 => Key::KEY_KP8,
        73 => Key::KEY_KP9,
        74 => Key::KEY_KPMINUS,
        75 => Key::KEY_KP4,
        76 => Key::KEY_KP5,
        77 => Key::KEY_KP6,
        78 => Key::KEY_KPPLUS,
        79 => Key::KEY_KP1,
        80 => Key::KEY_KP2,
        81 => Key::KEY_KP3,
        82 => Key::KEY_KP0,
        83 => Key::KEY_KPDOT,
        85 => Key::KEY_ZENKAKUHANKAKU,
        86 => Key::KEY_102ND,
        87 => Key::KEY_F11,
        88 => Key::KEY_F12,
        96 => Key::KEY_KPENTER,
        97 => Key::KEY_RIGHTCTRL,
        98 => Key::KEY_KPSLASH,
        99 => Key::KEY_SYSRQ,
        100 => Key::KEY_RIGHTALT,
        102 => Key::KEY_HOME,
        103 => Key::KEY_UP,
        104 => Key::KEY_PAGEUP,
        105 => Key::KEY_LEFT,
        106 => Key::KEY_RIGHT,
        107 => Key::KEY_END,
        108 => Key::KEY_DOWN,
        109 => Key::KEY_PAGEDOWN,
        110 => Key::KEY_INSERT,
        111 => Key::KEY_DELETE,
        113 => Key::KEY_MUTE,
        114 => Key::KEY_VOLUMEDOWN,
        115 => Key::KEY_VOLUMEUP,
        116 => Key::KEY_POWER,
        117 => Key::KEY_KPEQUAL,
        119 => Key::KEY_PAUSE,
        121 => Key::KEY_KPCOMMA,
        122 => Key::KEY_HANGEUL,
        123 => Key::KEY_HANJA,
        124 => Key::KEY_YEN,
        125 => Key::KEY_LEFTMETA,
        126 => Key::KEY_RIGHTMETA,
        127 => Key::KEY_COMPOSE,
        128 => Key::KEY_STOP,
        129 => Key::KEY_AGAIN,
        130 => Key::KEY_PROPS,
        131 => Key::KEY_UNDO,
        132 => Key::KEY_FRONT,
        133 => Key::KEY_COPY,
        134 => Key::KEY_OPEN,
        135 => Key::KEY_PASTE,
        136 => Key::KEY_FIND,
        137 => Key::KEY_CUT,
        138 => Key::KEY_HELP,
        139 => Key::KEY_MENU,
        140 => Key::KEY_CALC,
        141 => Key::KEY_SETUP,
        142 => Key::KEY_SLEEP,
        143 => Key::KEY_WAKEUP,
        144 => Key::KEY_FILE,
        145 => Key::KEY_SENDFILE,
        146 => Key::KEY_DELETEFILE,
        147 => Key::KEY_XFER,
        148 => Key::KEY_PROG1,
        149 => Key::KEY_PROG2,
        150 => Key::KEY_WWW,
        151 => Key::KEY_MSDOS,
        152 => Key::KEY_COFFEE,
        153 => Key::KEY_DIRECTION,
        154 => Key::KEY_CYCLEWINDOWS,
        155 => Key::KEY_MAIL,
        156 => Key::KEY_BOOKMARKS,
        157 => Key::KEY_COMPUTER,
        158 => Key::KEY_BACK,
        159 => Key::KEY_FORWARD,
        160 => Key::KEY_CLOSECD,
        161 => Key::KEY_EJECTCD,
        162 => Key::KEY_EJECTCLOSECD,
        163 => Key::KEY_NEXTSONG,
        164 => Key::KEY_PLAYPAUSE,
        165 => Key::KEY_PREVIOUSSONG,
        166 => Key::KEY_STOPCD,
        167 => Key::KEY_RECORD,
        168 => Key::KEY_REWIND,
        169 => Key::KEY_PHONE,
        170 => Key::KEY_ISO,
        171 => Key::KEY_CONFIG,
        172 => Key::KEY_HOMEPAGE,
        173 => Key::KEY_REFRESH,
        174 => Key::KEY_EXIT,
        175 => Key::KEY_MOVE,
        176 => Key::KEY_EDIT,
        177 => Key::KEY_SCROLLUP,
        178 => Key::KEY_SCROLLDOWN,
        179 => Key::KEY_KPLEFTPAREN,
        180 => Key::KEY_KPRIGHTPAREN,
        181 => Key::KEY_NEW,
        182 => Key::KEY_REDO,
        183 => Key::KEY_F13,
        184 => Key::KEY_F14,
        185 => Key::KEY_F15,
        186 => Key::KEY_F16,
        187 => Key::KEY_F17,
        188 => Key::KEY_F18,
        189 => Key::KEY_F19,
        190 => Key::KEY_F20,
        191 => Key::KEY_F21,
        192 => Key::KEY_F22,
        193 => Key::KEY_F23,
        194 => Key::KEY_F24,
        _ => return None,
    })
}

fn mouse_button_to_evdev(button: MouseButton) -> Key {
    match button {
        MouseButton::Left => Key::BTN_LEFT,
        MouseButton::Right => Key::BTN_RIGHT,
        MouseButton::Middle => Key::BTN_MIDDLE,
        MouseButton::X1 => Key::BTN_SIDE,
        MouseButton::X2 => Key::BTN_EXTRA,
    }
}

fn media_key_to_evdev(key: MediaKey) -> Key {
    match key {
        MediaKey::PlayPause => Key::KEY_PLAYPAUSE,
        MediaKey::NextTrack => Key::KEY_NEXTSONG,
        MediaKey::PreviousTrack => Key::KEY_PREVIOUSSONG,
        MediaKey::Stop => Key::KEY_STOPCD,
        MediaKey::VolumeUp => Key::KEY_VOLUMEUP,
        MediaKey::VolumeDown => Key::KEY_VOLUMEDOWN,
        MediaKey::Mute => Key::KEY_MUTE,
        MediaKey::Mail => Key::KEY_MAIL,
        MediaKey::Calculator => Key::KEY_CALC,
        MediaKey::Explorer => Key::KEY_COMPUTER,
    }
}

#[derive(Debug)]
pub struct LinuxBackend {
    config: BackendConfig,
    initialized: bool,
    keyboard: Option<Mutex<VirtualDevice>>,
    mouse: Option<Mutex<VirtualDevice>>,
    pressed_keys: Mutex<HashSet<u32>>,
}

impl LinuxBackend {
    pub fn new() -> Self {
        Self {
            config: BackendConfig::default(),
            initialized: false,
            keyboard: None,
            mouse: None,
            pressed_keys: Mutex::new(HashSet::new()),
        }
    }

    fn send_event(device: &VirtualDevice, event_type: EventType, code: u16, value: i32) -> Result<()> {
        device.emit(&[InputEvent::new(event_type, code, value)])
            .map_err(|e| PlatformError::Other(format!("evdev emit failed: {e}")))?;
        Ok(())
    }

    fn send_key(device: &VirtualDevice, code: u16, down: bool) -> Result<()> {
        Self::send_event(device, EventType::KEY, code, if down { 1 } else { 0 })
    }

    fn sync(device: &VirtualDevice) -> Result<()> {
        Self::send_event(device, EventType::SYNCHRONIZATION, 0, 0)
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
        "Linux (evdev)"
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn initialize(&mut self, config: BackendConfig) -> Result<()> {
        self.config = config;

        // Build keyboard attribute set with all keys we might use
        let mut keys = AttributeSet::<Key>::new();
        for i in 0..256 {
            if let Some(k) = keycode_to_evdev(i) {
                keys.insert(k);
            }
        }
        keys.insert(Key::KEY_PLAYPAUSE);
        keys.insert(Key::KEY_NEXTSONG);
        keys.insert(Key::KEY_PREVIOUSSONG);
        keys.insert(Key::KEY_STOPCD);
        keys.insert(Key::KEY_VOLUMEUP);
        keys.insert(Key::KEY_VOLUMEDOWN);
        keys.insert(Key::KEY_MUTE);
        keys.insert(Key::KEY_MAIL);
        keys.insert(Key::KEY_CALC);
        keys.insert(Key::KEY_COMPUTER);

        let keyboard = VirtualDeviceBuilder::new()
            .map_err(|e| PlatformError::Other(format!("evdev builder failed: {e}")))?
            .name(EVDEV_KEYBOARD_NAME)
            .with_keys(&keys)
            .map_err(|e| PlatformError::Other(format!("evdev keyboard setup failed: {e}")))?
            .build()
            .map_err(|e| PlatformError::Other(format!("evdev keyboard build failed: {e}")))?;

        // Build mouse with relative axes and buttons
        let mouse = VirtualDeviceBuilder::new()
            .map_err(|e| PlatformError::Other(format!("evdev builder failed: {e}")))?
            .name(EVDEV_MOUSE_NAME)
            .with_relative_axes(&AttributeSet::from_iter([
                RelativeAxisType::REL_X,
                RelativeAxisType::REL_Y,
                RelativeAxisType::REL_WHEEL,
                RelativeAxisType::REL_HWHEEL,
            ]))
            .with_keys(&AttributeSet::from_iter([
                Key::BTN_LEFT,
                Key::BTN_RIGHT,
                Key::BTN_MIDDLE,
                Key::BTN_SIDE,
                Key::BTN_EXTRA,
            ]))
            .map_err(|e| PlatformError::Other(format!("evdev mouse setup failed: {e}")))?
            .build()
            .map_err(|e| PlatformError::Other(format!("evdev mouse build failed: {e}")))?;

        self.keyboard = Some(Mutex::new(keyboard));
        self.mouse = Some(Mutex::new(mouse));
        self.initialized = true;
        info!("Linux backend initialized with evdev uinput devices");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Release all pressed keys
        if let Some(ref kb) = self.keyboard {
            if let Ok(kb) = kb.lock() {
                let keys = self.pressed_keys.lock().unwrap_or_else(|e| e.into_inner());
                for &k in keys.iter() {
                    if let Some(ev) = keycode_to_evdev(k) {
                        let _ = Self::send_key(&kb, ev.code(), false);
                    }
                }
                let _ = Self::sync(&kb);
            }
        }
        self.keyboard = None;
        self.mouse = None;
        self.initialized = false;
        info!("Linux backend shut down");
        Ok(())
    }

    async fn send_controller_state(&self, state: &ControllerState) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        debug!("controller state: id={}, left_stick=({},{})", state.id, state.left_stick_x, state.left_stick_y);
        Ok(())
    }

    async fn send_key_press(&self, key: u32, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        let kb = self.keyboard.as_ref()
            .ok_or_else(|| PlatformError::Other("keyboard not initialized".into()))?;
        let kb = kb.lock().map_err(|e| PlatformError::Other(format!("lock: {e}")))?;

        let ev_key = keycode_to_evdev(key)
            .ok_or_else(|| PlatformError::Other(format!("unknown keycode: {key}")))?;

        if down {
            self.pressed_keys.lock().unwrap_or_else(|e| e.into_inner()).insert(key);
        } else {
            self.pressed_keys.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
        }

        Self::send_key(&kb, ev_key.code(), down)?;
        Self::sync(&kb)?;
        debug!("key press: key={}, down={}", key, down);
        Ok(())
    }

    async fn send_mouse_move(&self, dx: i32, dy: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        let mouse = self.mouse.as_ref()
            .ok_or_else(|| PlatformError::Other("mouse not initialized".into()))?;
        let mouse = mouse.lock().map_err(|e| PlatformError::Other(format!("lock: {e}")))?;

        Self::send_event(&mouse, EventType::RELATIVE, RelativeAxisType::REL_X.code(), dx)?;
        Self::send_event(&mouse, EventType::RELATIVE, RelativeAxisType::REL_Y.code(), dy)?;
        Self::sync(&mouse)?;
        debug!("mouse move: dx={}, dy={}", dx, dy);
        Ok(())
    }

    async fn send_mouse_button(&self, button: MouseButton, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        let mouse = self.mouse.as_ref()
            .ok_or_else(|| PlatformError::Other("mouse not initialized".into()))?;
        let mouse = mouse.lock().map_err(|e| PlatformError::Other(format!("lock: {e}")))?;

        let ev_key = mouse_button_to_evdev(button);
        Self::send_key(&mouse, ev_key.code(), down)?;
        Self::sync(&mouse)?;
        debug!("mouse button: {:?}, down={}", button, down);
        Ok(())
    }

    async fn send_mouse_scroll(&self, x: i32, y: i32) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        let mouse = self.mouse.as_ref()
            .ok_or_else(|| PlatformError::Other("mouse not initialized".into()))?;
        let mouse = mouse.lock().map_err(|e| PlatformError::Other(format!("lock: {e}")))?;

        if x != 0 {
            Self::send_event(&mouse, EventType::RELATIVE, RelativeAxisType::REL_HWHEEL.code(), x)?;
        }
        if y != 0 {
            Self::send_event(&mouse, EventType::RELATIVE, RelativeAxisType::REL_WHEEL.code(), -y)?;
        }
        Self::sync(&mouse)?;
        debug!("mouse scroll: x={}, y={}", x, y);
        Ok(())
    }

    async fn send_media_key(&self, key: MediaKey, down: bool) -> Result<()> {
        if !self.initialized {
            return Err(PlatformError::NotInitialized);
        }
        let kb = self.keyboard.as_ref()
            .ok_or_else(|| PlatformError::Other("keyboard not initialized".into()))?;
        let kb = kb.lock().map_err(|e| PlatformError::Other(format!("lock: {e}")))?;

        let ev_key = media_key_to_evdev(key);
        Self::send_key(&kb, ev_key.code(), down)?;
        Self::sync(&kb)?;
        debug!("media key: {:?}, down={}", key, down);
        Ok(())
    }

    fn supported_features(&self) -> Vec<&str> {
        vec!["keyboard", "mouse", "media"]
    }
}
