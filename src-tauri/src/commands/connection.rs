use tauri::State;

use crate::app::AppManager;

#[tauri::command]
pub async fn connect_device(
    manager: State<'_, AppManager>,
    address: String,
    port: u16,
) -> Result<String, String> {
    manager
        .connect_device(&address, port)
        .await
        .map_err(|e| e.to_string())?;

    Ok("connected".to_string())
}

#[tauri::command]
pub async fn disconnect_device(manager: State<'_, AppManager>) -> Result<String, String> {
    manager
        .disconnect_device()
        .await
        .map_err(|e| e.to_string())?;

    Ok("disconnected".to_string())
}

#[tauri::command]
pub async fn get_connection_status(manager: State<'_, AppManager>) -> Result<String, String> {
    Ok(manager
        .get_connection_status()
        .await
        .unwrap_or_else(|| "disconnected".to_string()))
}

#[tauri::command]
pub async fn get_connected_devices(
    manager: State<'_, AppManager>,
) -> Result<Vec<String>, String> {
    let status = manager.get_connection_status().await;
    match status {
        Some(s) => Ok(vec![s]),
        None => Ok(Vec::new()),
    }
}
