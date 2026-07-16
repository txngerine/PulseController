#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod ui;

use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use tauri::Manager;

fn main() {
    // Initialize logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("PulsePad starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app::AppManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::connection::connect_device,
            commands::connection::disconnect_device,
            commands::connection::get_connection_status,
            commands::connection::get_connected_devices,
            commands::profiles::list_profiles,
            commands::profiles::get_profile,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            commands::profiles::set_active_profile,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            commands::telemetry::get_metrics,
            commands::telemetry::reset_metrics,
            commands::discovery::start_discovery,
            commands::discovery::stop_discovery,
            commands::discovery::get_discovered_devices,
            commands::logs::get_logs,
            commands::logs::clear_logs,
        ])
        .setup(|app| {
            let manager = app.state::<app::AppManager>();
            let manager_clone = manager.inner().clone();

            // Initialize the app in a background task
            tauri::async_runtime::spawn(async move {
                if let Err(e) = manager_clone.initialize().await {
                    tracing::error!("failed to initialize app: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
