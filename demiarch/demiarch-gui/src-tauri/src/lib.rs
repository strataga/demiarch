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

// ============================================================================
// Conflict Resolution Types
// ============================================================================

/// A file with detected conflicts between generated and user versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFile {
    /// Relative file path from project root
    pub file_path: String,
    /// Current status: "modified", "deleted", or "unchanged"
    pub status: String,
    /// Original content hash when generated
    pub original_hash: String,
    /// Current content hash (None if deleted)
    pub current_hash: Option<String>,
    /// Original content from checkpoint (for diff)
    pub original_content: Option<String>,
    /// Current content on disk (for diff)
    pub current_content: Option<String>,
    /// When the file was originally generated (ISO 8601)
    pub generated_at: Option<String>,
    /// Feature that generated this file (if any)
    pub feature_id: Option<String>,
}

/// Summary of edit detection across project files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSummary {
    pub total_files: usize,
    pub modified_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub unchanged_files: Vec<String>,
}

/// Resolution strategy for a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    #[serde(rename = "keep-user")]
    KeepUser,
    #[serde(rename = "keep-generated")]
    KeepGenerated,
    #[serde(rename = "merge")]
    Merge,
}

/// Result of resolving a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    pub file_path: String,
    pub strategy: String,
    pub success: bool,
    pub error: Option<String>,
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

// ============================================================================
// Conflict Resolution Commands
// ============================================================================

/// Check all tracked files for conflicts (user edits to generated code)
#[tauri::command]
async fn check_for_conflicts(_project_id: String) -> Result<ConflictSummary, String> {
    // TODO: Integrate with demiarch_core::domain::recovery::EditDetectionService
    // For now, return mock data for UI development
    Ok(ConflictSummary {
        total_files: 5,
        modified_files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        deleted_files: vec!["src/unused.rs".to_string()],
        unchanged_files: vec!["src/config.rs".to_string(), "Cargo.toml".to_string()],
    })
}

/// Get detailed conflict information for files with edits
#[tauri::command]
async fn get_conflict_details(_project_id: String) -> Result<Vec<ConflictFile>, String> {
    // TODO: Integrate with demiarch_core::domain::recovery::EditDetectionService
    // For now, return mock data for UI development
    Ok(vec![
        ConflictFile {
            file_path: "src/main.rs".to_string(),
            status: "modified".to_string(),
            original_hash: "abc123".to_string(),
            current_hash: Some("def456".to_string()),
            original_content: Some("fn main() {\n    println!(\"Hello, world!\");\n}".to_string()),
            current_content: Some(
                "fn main() {\n    println!(\"Hello, Demiarch!\");\n    // User added this comment\n    init_app();\n}"
                    .to_string(),
            ),
            generated_at: Some("2024-01-15T10:30:00Z".to_string()),
            feature_id: None,
        },
        ConflictFile {
            file_path: "src/lib.rs".to_string(),
            status: "modified".to_string(),
            original_hash: "ghi789".to_string(),
            current_hash: Some("jkl012".to_string()),
            original_content: Some("pub mod app;\npub mod config;".to_string()),
            current_content: Some(
                "pub mod app;\npub mod config;\npub mod utils; // User added module".to_string(),
            ),
            generated_at: Some("2024-01-15T10:30:00Z".to_string()),
            feature_id: None,
        },
        ConflictFile {
            file_path: "src/unused.rs".to_string(),
            status: "deleted".to_string(),
            original_hash: "mno345".to_string(),
            current_hash: None,
            original_content: Some("// This file was deleted by user".to_string()),
            current_content: None,
            generated_at: Some("2024-01-14T09:00:00Z".to_string()),
            feature_id: None,
        },
    ])
}

/// Resolve a single conflict with the specified strategy
#[tauri::command]
async fn resolve_conflict(
    file_path: String,
    strategy: String,
) -> Result<ResolutionResult, String> {
    // TODO: Integrate with demiarch_core::domain::recovery
    // - "keep-user": Acknowledge user edits as new baseline
    // - "keep-generated": Restore from checkpoint
    // - "merge": Future feature for intelligent merging

    Ok(ResolutionResult {
        file_path,
        strategy,
        success: true,
        error: None,
    })
}

/// Acknowledge user edits (accept as new baseline without restoring)
#[tauri::command]
async fn acknowledge_edits(
    _project_id: String,
    file_paths: Vec<String>,
) -> Result<usize, String> {
    // TODO: Integrate with demiarch_core::domain::recovery::EditDetectionService::acknowledge_edits
    Ok(file_paths.len())
}

/// Get diff between original and current content for a specific file
#[tauri::command]
async fn get_file_diff(
    _project_id: String,
    _file_path: String,
) -> Result<ConflictFile, String> {
    // TODO: Integrate with demiarch_core to read actual file content
    // and compare with tracked original hash

    Err("File not found or not tracked".to_string())
}

/// Run the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            health_check,
            greet,
            check_for_conflicts,
            get_conflict_details,
            resolve_conflict,
            acknowledge_edits,
            get_file_diff,
        ])
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
