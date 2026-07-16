use tauri::State;
use uuid::Uuid;

use crate::app::AppManager;

#[tauri::command]
pub async fn list_profiles(manager: State<'_, AppManager>) -> Result<String, String> {
    let profiles = manager.inner.profiles.read().await;
    let profiles_json: Vec<&pulsepad_profiles::schema::Profile> = profiles.list_profiles();
    serde_json::to_string(&profiles_json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_profile(manager: State<'_, AppManager>, id: String) -> Result<String, String> {
    let profile_id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let profiles = manager.inner.profiles.read().await;
    let profile = profiles.get_profile(profile_id).ok_or("profile not found")?;
    serde_json::to_string(profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_profile(
    manager: State<'_, AppManager>,
    name: String,
    description: String,
) -> Result<String, String> {
    let mut profiles = manager.inner.profiles.write().await;
    let profile = profiles
        .create_profile(name, description)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_profile(
    manager: State<'_, AppManager>,
    id: String,
    profile_json: String,
) -> Result<String, String> {
    let profile_id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let updated_profile: pulsepad_profiles::schema::Profile =
        serde_json::from_str(&profile_json).map_err(|e| e.to_string())?;

    let mut profiles = manager.inner.profiles.write().await;
    if let Some(profile) = profiles.get_profile_mut(profile_id) {
        *profile = updated_profile.clone();
        profiles.save_profile(&updated_profile).await.map_err(|e| e.to_string())?;
        Ok("updated".to_string())
    } else {
        Err("profile not found".to_string())
    }
}

#[tauri::command]
pub async fn delete_profile(manager: State<'_, AppManager>, id: String) -> Result<String, String> {
    let profile_id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut profiles = manager.inner.profiles.write().await;
    profiles
        .delete_profile(profile_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok("deleted".to_string())
}

#[tauri::command]
pub async fn set_active_profile(
    manager: State<'_, AppManager>,
    id: String,
) -> Result<String, String> {
    let profile_id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut profiles = manager.inner.profiles.write().await;
    profiles
        .set_active_profile(profile_id)
        .map_err(|e| e.to_string())?;
    Ok("active".to_string())
}
