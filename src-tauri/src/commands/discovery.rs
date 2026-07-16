use tauri::State;

use crate::app::AppManager;

#[tauri::command]
pub async fn start_discovery(manager: State<'_, AppManager>) -> Result<String, String> {
    let discovery = manager.inner.discovery.read().await;
    discovery
        .start_udp_broadcast()
        .await
        .map_err(|e| e.to_string())?;
    Ok("started".to_string())
}

#[tauri::command]
pub async fn stop_discovery(manager: State<'_, AppManager>) -> Result<String, String> {
    let discovery = manager.inner.discovery.read().await;
    discovery.stop();
    Ok("stopped".to_string())
}

#[tauri::command]
pub async fn get_discovered_devices(manager: State<'_, AppManager>) -> Result<String, String> {
    let discovery = manager.inner.discovery.read().await;
    let devices = discovery.list_devices();
    serde_json::to_string(&devices).map_err(|e| e.to_string())
}
