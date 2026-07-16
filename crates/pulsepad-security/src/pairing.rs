use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: Uuid,
    pub name: String,
    pub device_id: String,
    pub paired_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub public_key: Option<String>,
    pub trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: [u8; 32],
    pub device_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub valid: bool,
}

impl SessionToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        self.valid && !self.is_expired()
    }
}

#[derive(Debug)]
pub struct PairingManager {
    trusted_devices: HashMap<Uuid, TrustedDevice>,
    sessions: HashMap<Uuid, SessionToken>,
    storage_path: PathBuf,
}

impl PairingManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            trusted_devices: HashMap::new(),
            sessions: HashMap::new(),
            storage_path,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        if self.storage_path.exists() {
            let content = tokio::fs::read_to_string(&self.storage_path).await?;
            let data: PairingData = serde_json::from_str(&content)?;
            self.trusted_devices = data.trusted_devices;
            info!("loaded {} trusted devices", self.trusted_devices.len());
        }
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        let data = PairingData {
            trusted_devices: self.trusted_devices.clone(),
        };
        let content = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.storage_path, content).await?;
        debug!("saved pairing data");
        Ok(())
    }

    pub fn generate_challenge(&self) -> [u8; 32] {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut challenge = [0u8; 32];
        rng.fill(&mut challenge);
        challenge
    }

    pub fn verify_response(
        &self,
        challenge: &[u8; 32],
        response: &[u8; 32],
    ) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // In a real implementation, this would use the device's public key
        // For now, use a simple HMAC verification
        let key = b"pulsepad-pairing-key";
        let mut mac = HmacSha256::new_from_slice(key).unwrap_or_else(|_| {
            panic!("HMAC can take key of any size");
        });
        mac.update(challenge);
        let result = mac.finalize();
        let expected = result.into_bytes();

        expected.as_slice() == response
    }

    pub fn create_session(&mut self, device_id: Uuid) -> Result<SessionToken> {
        use rand::Rng;

        let mut token = [0u8; 32];
        rand::thread_rng().fill(&mut token);

        let session = SessionToken {
            token,
            device_id,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            valid: true,
        };

        self.sessions.insert(device_id, session.clone());
        info!("created session for device {}", device_id);
        Ok(session)
    }

    pub fn validate_session(&self, device_id: Uuid, token: &[u8; 32]) -> bool {
        match self.sessions.get(&device_id) {
            Some(session) => session.token == *token && session.is_valid(),
            None => false,
        }
    }

    pub fn add_trusted_device(&mut self, device: TrustedDevice) -> Result<()> {
        self.trusted_devices.insert(device.id, device);
        info!("added trusted device");
        Ok(())
    }

    pub fn remove_trusted_device(&mut self, id: Uuid) -> Result<()> {
        self.trusted_devices.remove(&id);
        self.sessions.remove(&id);
        info!("removed trusted device {}", id);
        Ok(())
    }

    pub fn is_device_trusted(&self, device_id: &str) -> bool {
        self.trusted_devices
            .values()
            .any(|d| d.device_id == device_id && d.trusted)
    }

    pub fn get_trusted_device(&self, id: Uuid) -> Option<&TrustedDevice> {
        self.trusted_devices.get(&id)
    }

    pub fn list_trusted_devices(&self) -> Vec<&TrustedDevice> {
        self.trusted_devices.values().collect()
    }

    pub fn invalidate_session(&mut self, device_id: Uuid) {
        if let Some(session) = self.sessions.get_mut(&device_id) {
            session.valid = false;
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PairingData {
    trusted_devices: HashMap<Uuid, TrustedDevice>,
}
