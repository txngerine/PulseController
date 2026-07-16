use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::error::{Result, StorageError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralSettings,
    pub network: NetworkSettings,
    pub input: InputSettings,
    pub appearance: AppearanceSettings,
    pub advanced: AdvancedSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub auto_launch: bool,
    pub run_in_tray: bool,
    pub minimize_to_tray: bool,
    pub check_updates: bool,
    pub update_channel: UpdateChannel,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            auto_launch: false,
            run_in_tray: true,
            minimize_to_tray: true,
            check_updates: true,
            update_channel: UpdateChannel::Stable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum UpdateChannel {
    Stable = 0,
    Beta = 1,
    Dev = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub preferred_transport: String,
    pub udp_port: u16,
    pub udp_bind_address: String,
    pub bluetooth_enabled: bool,
    pub auto_discover: bool,
    pub discovery_port: u16,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            preferred_transport: "udp".to_string(),
            udp_port: 9876,
            udp_bind_address: "0.0.0.0".to_string(),
            bluetooth_enabled: true,
            auto_discover: true,
            discovery_port: 9877,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSettings {
    pub mouse_sensitivity: f32,
    pub mouse_acceleration: bool,
    pub keyboard_repeat_rate: u32,
    pub keyboard_repeat_delay: u32,
    pub default_deadzone: u16,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            mouse_acceleration: false,
            keyboard_repeat_rate: 30,
            keyboard_repeat_delay: 500,
            default_deadzone: 800,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub sidebar_collapsed: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            sidebar_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Theme {
    Dark = 0,
    Light = 1,
    Auto = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    pub debug_logging: bool,
    pub log_level: String,
    pub telemetry_enabled: bool,
    pub packet_buffer_size: usize,
    pub max_reconnect_attempts: u32,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            debug_logging: false,
            log_level: "info".to_string(),
            telemetry_enabled: true,
            packet_buffer_size: 4096,
            max_reconnect_attempts: 3,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            network: NetworkSettings::default(),
            input: InputSettings::default(),
            appearance: AppearanceSettings::default(),
            advanced: AdvancedSettings::default(),
        }
    }
}

#[derive(Debug)]
pub struct StorageManager {
    config_dir: PathBuf,
    data_dir: PathBuf,
    config: AppConfig,
}

impl StorageManager {
    pub fn new(app_name: &str) -> Result<Self> {
        let config_dir = Self::get_config_dir(app_name)
            .ok_or_else(|| StorageError::Directory("failed to get config directory".to_string()))?;

        let data_dir = Self::get_data_dir(app_name)
            .ok_or_else(|| StorageError::Directory("failed to get data directory".to_string()))?;

        Ok(Self {
            config_dir,
            data_dir,
            config: AppConfig::default(),
        })
    }

    fn get_config_dir(app_name: &str) -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join(app_name))
    }

    fn get_data_dir(app_name: &str) -> Option<PathBuf> {
        dirs::data_dir().map(|dir| dir.join(app_name))
    }

    pub async fn initialize(&mut self) -> Result<()> {
        tokio::fs::create_dir_all(&self.config_dir).await?;
        tokio::fs::create_dir_all(&self.data_dir).await?;

        let profiles_dir = self.data_dir.join("profiles");
        tokio::fs::create_dir_all(&profiles_dir).await?;

        match self.load_config().await {
            Ok(config) => {
                self.config = config;
                info!("loaded config from {:?}", self.config_path());
            }
            Err(_) => {
                self.save_config().await?;
                info!("created default config at {:?}", self.config_path());
            }
        }

        Ok(())
    }

    fn config_path(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    pub async fn load_config(&mut self) -> Result<AppConfig> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(AppConfig::default());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub async fn save_config(&self) -> Result<()> {
        let path = self.config_path();
        let content = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::write(path, content).await?;
        debug!("saved config");
        Ok(())
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.data_dir.join("profiles")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }
}
