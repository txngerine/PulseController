use tauri::State;

use crate::app::AppManager;

#[tauri::command]
pub async fn get_settings(manager: State<'_, AppManager>) -> Result<String, String> {
    let storage = manager.inner.storage.read().await;
    let config = storage.config();
    serde_json::to_string(config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    manager: State<'_, AppManager>,
    settings_json: String,
) -> Result<String, String> {
    let new_settings: pulsepad_storage::AppConfig =
        serde_json::from_str(&settings_json).map_err(|e| e.to_string())?;

    let mut storage = manager.inner.storage.write().await;
    *storage.config_mut() = new_settings;
    storage.save_config().await.map_err(|e| e.to_string())?;
    Ok("updated".to_string())
}

#[tauri::command]
pub async fn reset_settings(manager: State<'_, AppManager>) -> Result<String, String> {
    let mut storage = manager.inner.storage.write().await;
    *storage.config_mut() = pulsepad_storage::AppConfig::default();
    storage.save_config().await.map_err(|e| e.to_string())?;
    Ok("reset".to_string())
}
