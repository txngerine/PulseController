use std::path::PathBuf;
use tracing::{error, info};

use pulsepad_profiles::ProfileManager;

pub struct ProfilesManager {
    manager: ProfileManager,
}

impl ProfilesManager {
    pub fn new(profiles_dir: PathBuf) -> Self {
        Self {
            manager: ProfileManager::new(profiles_dir),
        }
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        self.manager.load_all().await?;
        info!("profiles manager initialized");
        Ok(())
    }

    pub fn get_manager(&self) -> &ProfileManager {
        &self.manager
    }

    pub fn get_manager_mut(&mut self) -> &mut ProfileManager {
        &mut self.manager
    }
}
