//! Demiarch GUI Library
//!
//! Tauri command implementations and application entry point
//! for the Demiarch GUI interface.

use serde::{Deserialize, Serialize};
use tauri::Manager;

// Re-export core functionality
pub use demiarch_core;

/// Application state shared across all windows
#[derive(Debug, Default)]
pub struct AppState {
    pub initialized: std::sync::atomic::AtomicBool,
}

/// Application info returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub message: String,
}

/// Get application information
#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Demiarch".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Local-first AI app builder".to_string(),
    }
}

/// Perform a health check
#[tauri::command]
fn health_check() -> HealthStatus {
    HealthStatus {
        status: "healthy".to_string(),
        message: "Demiarch GUI is running".to_string(),
    }
}

/// Greet command for testing IPC
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Demiarch.", name)
}

/// Run the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![get_app_info, health_check, greet,])
        .setup(|app| {
            let state = app.state::<AppState>();
            state
                .initialized
                .store(true, std::sync::atomic::Ordering::SeqCst);

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
