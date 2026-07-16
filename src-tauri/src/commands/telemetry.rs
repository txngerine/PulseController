use tauri::State;

use crate::app::AppManager;

#[tauri::command]
pub async fn get_metrics(manager: State<'_, AppManager>) -> Result<String, String> {
    let telemetry = manager.inner.telemetry.read().await;
    let snapshot = telemetry.snapshot();
    serde_json::to_string(&snapshot).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_metrics(manager: State<'_, AppManager>) -> Result<String, String> {
    let telemetry = manager.inner.telemetry.read().await;
    telemetry.reset();
    Ok("reset".to_string())
}
