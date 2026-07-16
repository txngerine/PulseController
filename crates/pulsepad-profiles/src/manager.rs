use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{ProfileError, Result};
use crate::schema::Profile;

#[derive(Debug)]
pub struct ProfileManager {
    profiles: HashMap<Uuid, Profile>,
    active_profile: Option<Uuid>,
    profiles_dir: PathBuf,
}

impl ProfileManager {
    pub fn new(profiles_dir: PathBuf) -> Self {
        Self {
            profiles: HashMap::new(),
            active_profile: None,
            profiles_dir,
        }
    }

    pub async fn load_all(&mut self) -> Result<()> {
        if !self.profiles_dir.exists() {
            tokio::fs::create_dir_all(&self.profiles_dir).await?;
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.profiles_dir).await?;
        let mut loaded = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match self.load_profile_from_path(&path).await {
                    Ok(profile) => {
                        self.profiles.insert(profile.id, profile);
                        loaded += 1;
                    }
                    Err(e) => {
                        warn!("failed to load profile from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("loaded {} profiles", loaded);
        Ok(())
    }

    async fn load_profile_from_path(&self, path: &Path) -> Result<Profile> {
        let content = tokio::fs::read_to_string(path).await?;
        let profile: Profile = serde_json::from_str(&content)?;
        Ok(profile)
    }

    pub async fn save_profile(&self, profile: &Profile) -> Result<()> {
        let path = self.profiles_dir.join(format!("{}.json", profile.id));
        let content = serde_json::to_string_pretty(profile)?;
        tokio::fs::write(path, content).await?;
        debug!("saved profile: {}", profile.name);
        Ok(())
    }

    pub async fn create_profile(&mut self, name: String, description: String) -> Result<Profile> {
        let profile = Profile::new(name, description);
        self.save_profile(&profile).await?;
        self.profiles.insert(profile.id, profile.clone());
        info!("created profile: {}", profile.name);
        Ok(profile)
    }

    pub async fn delete_profile(&mut self, id: Uuid) -> Result<()> {
        let profile_name = self.profiles.get(&id)
            .map(|p| p.name.clone())
            .ok_or(ProfileError::NotFound(id.to_string()))?;

        let path = self.profiles_dir.join(format!("{}.json", id));
        tokio::fs::remove_file(path).await?;
        self.profiles.remove(&id);

        if self.active_profile == Some(id) {
            self.active_profile = None;
        }

        info!("deleted profile: {}", profile_name);
        Ok(())
    }

    pub fn get_profile(&self, id: Uuid) -> Option<&Profile> {
        self.profiles.get(&id)
    }

    pub fn get_profile_mut(&mut self, id: Uuid) -> Option<&mut Profile> {
        self.profiles.get_mut(&id)
    }

    pub fn list_profiles(&self) -> Vec<&Profile> {
        self.profiles.values().collect()
    }

    pub fn set_active_profile(&mut self, id: Uuid) -> Result<()> {
        if !self.profiles.contains_key(&id) {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        self.active_profile = Some(id);
        info!("active profile set to {}", id);
        Ok(())
    }

    pub fn get_active_profile(&self) -> Option<&Profile> {
        self.active_profile
            .and_then(|id| self.profiles.get(&id))
    }

    pub fn get_active_profile_id(&self) -> Option<Uuid> {
        self.active_profile
    }

    pub fn get_default_profile(&self) -> Option<&Profile> {
        self.profiles.values().find(|p| p.is_default)
    }

    pub async fn set_default_profile(&mut self, id: Uuid) -> Result<()> {
        let ids_to_save: Vec<Uuid> = self.profiles.values()
            .filter(|p| p.is_default)
            .map(|p| p.id)
            .collect();
        for profile_id in ids_to_save {
            if let Some(profile) = self.profiles.get(&profile_id) {
                let profile_clone = profile.clone();
                self.save_profile(&profile_clone).await?;
            }
            if let Some(profile) = self.profiles.get_mut(&profile_id) {
                profile.is_default = false;
            }
        }

        let profile = self.profiles.get_mut(&id).ok_or(ProfileError::NotFound(
            id.to_string(),
        ))?;

        profile.is_default = true;
        let profile_name = profile.name.clone();
        let profile_clone = profile.clone();
        let _ = profile;
        self.save_profile(&profile_clone).await?;
        info!("set default profile: {}", profile_name);
        Ok(())
    }
}
