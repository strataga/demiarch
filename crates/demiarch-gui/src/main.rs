//! Demiarch GUI - Tauri Desktop Application
//!
//! This is the main entry point for the Demiarch desktop GUI.
//! It provides a native application shell that hosts the React frontend.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_projects,
            commands::get_project,
            commands::create_project,
            commands::delete_project,
            commands::get_features,
            commands::get_feature,
            commands::update_feature_status,
            commands::get_sessions,
            commands::get_costs,
            commands::get_agents,
            commands::doctor,
            commands::get_conflicts,
            commands::resolve_conflict_hunk,
            commands::apply_conflict_resolutions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
