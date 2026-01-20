//! Demiarch GUI Library
//!
//! Shared GUI functionality and Tauri command implementations
//! for the Demiarch GUI interface.

use tauri::Manager;

// Re-export core functionality
pub use demiarch_core;

/// Run the Tauri application
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            println!("Demiarch GUI initialized");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
