//! Demiarch GUI Library
//!
//! Tauri command implementations and application entry point
//! for the Demiarch GUI interface.

use std::path::PathBuf;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, FromRow, Row, SqlitePool};
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;
use uuid::Uuid;

use std::sync::Arc;

/// Application state shared across all windows
pub struct AppState {
    pub initialized: std::sync::atomic::AtomicBool,
    pub pool: Arc<RwLock<Option<SqlitePool>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            initialized: std::sync::atomic::AtomicBool::new(false),
            pool: Arc::new(RwLock::new(None)),
        }
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("initialized", &self.initialized)
            .finish()
    }
}

/// Get the default demiarch database path
fn get_db_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("demiarch")
        .join("demiarch.db")
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
// Project & Feature Types
// ============================================================================

/// Project info returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub repo_url: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Feature info returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureInfo {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i32,
    pub labels: Option<Vec<String>>,
    pub acceptance_criteria: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Agent Event Types (mirrors demiarch-core::agents::events)
// ============================================================================

/// Agent event file path
fn get_agent_events_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".demiarch")
        .join("agent-events.jsonl")
}

/// Type of agent event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    Spawned,
    Started,
    StatusUpdate,
    Completed,
    Failed,
    Cancelled,
    TokenUpdate,
    Disposed,
}

/// Agent data included in events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventData {
    pub id: String,
    pub agent_type: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub path: String,
    pub status: String,
    pub tokens: u64,
    pub task: Option<String>,
    pub error: Option<String>,
}

/// Agent lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub timestamp: DateTime<Utc>,
    pub event_id: Uuid,
    pub session_id: Uuid,
    pub event_type: AgentEventType,
    pub agent: AgentEventData,
}

// ============================================================================
// Conflict Resolution Types
// ============================================================================

/// A file with detected conflicts between generated and user versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFile {
    pub file_path: String,
    pub status: String,
    pub original_hash: String,
    pub current_hash: Option<String>,
    pub original_content: Option<String>,
    pub current_content: Option<String>,
    pub generated_at: Option<String>,
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

/// Result of resolving a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    pub file_path: String,
    pub strategy: String,
    pub success: bool,
    pub error: Option<String>,
}

// ============================================================================
// Basic Commands
// ============================================================================

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
// Project & Feature Commands
// ============================================================================

/// List all projects
#[tauri::command]
async fn list_projects(state: tauri::State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    let pool_guard: tokio::sync::RwLockReadGuard<'_, Option<SqlitePool>> = state.pool.read().await;
    let pool: &SqlitePool = pool_guard.as_ref().ok_or("Database not initialized")?;

    let projects: Vec<ProjectInfo> = sqlx::query_as::<_, ProjectInfo>(
        r#"
        SELECT
            id,
            name,
            framework,
            repo_url,
            status,
            description,
            datetime(created_at) as created_at,
            datetime(updated_at) as updated_at
        FROM projects
        WHERE status = 'active'
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list projects: {}", e))?;

    Ok(projects)
}

/// Get a specific project by ID
#[tauri::command]
async fn get_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<ProjectInfo, String> {
    let pool_guard: tokio::sync::RwLockReadGuard<'_, Option<SqlitePool>> = state.pool.read().await;
    let pool: &SqlitePool = pool_guard.as_ref().ok_or("Database not initialized")?;

    let project: ProjectInfo = sqlx::query_as::<_, ProjectInfo>(
        r#"
        SELECT
            id,
            name,
            framework,
            repo_url,
            status,
            description,
            datetime(created_at) as created_at,
            datetime(updated_at) as updated_at
        FROM projects
        WHERE id = ?
        "#,
    )
    .bind(&project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get project: {}", e))?
    .ok_or("Project not found")?;

    Ok(project)
}

/// List features for a project
#[tauri::command]
async fn list_features(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<FeatureInfo>, String> {
    let pool_guard: tokio::sync::RwLockReadGuard<'_, Option<SqlitePool>> = state.pool.read().await;
    let pool: &SqlitePool = pool_guard.as_ref().ok_or("Database not initialized")?;

    let rows = sqlx::query(
        r#"
        SELECT
            id,
            project_id,
            title,
            description,
            status,
            priority,
            labels,
            acceptance_criteria,
            datetime(created_at) as created_at,
            datetime(updated_at) as updated_at
        FROM features
        WHERE project_id = ?
        ORDER BY priority ASC, created_at DESC
        "#,
    )
    .bind(&project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list features: {}", e))?;

    let features: Vec<FeatureInfo> = rows
        .into_iter()
        .map(|row: sqlx::sqlite::SqliteRow| {
            let labels_str: Option<String> = row.get("labels");
            let labels = labels_str.map(|s: String| {
                serde_json::from_str::<Vec<String>>(&s).unwrap_or_else(|_| {
                    s.split(',').map(|l| l.trim().to_string()).collect()
                })
            });

            FeatureInfo {
                id: row.get("id"),
                project_id: row.get("project_id"),
                title: row.get("title"),
                description: row.get("description"),
                status: row.get("status"),
                priority: row.get("priority"),
                labels,
                acceptance_criteria: row.get("acceptance_criteria"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        })
        .collect();

    Ok(features)
}

/// Update a feature's status
#[tauri::command]
async fn update_feature_status(
    state: tauri::State<'_, AppState>,
    feature_id: String,
    status: String,
) -> Result<(), String> {
    let pool_guard: tokio::sync::RwLockReadGuard<'_, Option<SqlitePool>> = state.pool.read().await;
    let pool: &SqlitePool = pool_guard.as_ref().ok_or("Database not initialized")?;

    // Validate status
    let valid_statuses = ["backlog", "todo", "in_progress", "review", "done"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(format!(
            "Invalid status: {}. Valid values: {:?}",
            status, valid_statuses
        ));
    }

    sqlx::query(
        r#"
        UPDATE features
        SET status = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&status)
    .bind(&feature_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update feature status: {}", e))?;

    Ok(())
}

// ============================================================================
// Conflict Resolution Commands
// ============================================================================

/// Check all tracked files for conflicts (user edits to generated code)
#[tauri::command]
async fn check_for_conflicts(_project_id: String) -> Result<ConflictSummary, String> {
    // TODO: Integrate with actual edit detection
    Ok(ConflictSummary {
        total_files: 0,
        modified_files: vec![],
        deleted_files: vec![],
        unchanged_files: vec![],
    })
}

/// Get detailed conflict information for files with edits
#[tauri::command]
async fn get_conflict_details(_project_id: String) -> Result<Vec<ConflictFile>, String> {
    // TODO: Integrate with actual edit detection
    Ok(vec![])
}

/// Resolve a single conflict with the specified strategy
#[tauri::command]
async fn resolve_conflict(file_path: String, strategy: String) -> Result<ResolutionResult, String> {
    Ok(ResolutionResult {
        file_path,
        strategy,
        success: true,
        error: None,
    })
}

/// Acknowledge user edits (accept as new baseline without restoring)
#[tauri::command]
async fn acknowledge_edits(_project_id: String, file_paths: Vec<String>) -> Result<usize, String> {
    Ok(file_paths.len())
}

/// Get diff between original and current content for a specific file
#[tauri::command]
async fn get_file_diff(_project_id: String, _file_path: String) -> Result<ConflictFile, String> {
    Err("File not found or not tracked".to_string())
}

// ============================================================================
// Agent Event Streaming Commands
// ============================================================================

/// Start watching agent events and emit to frontend
#[tauri::command]
async fn start_agent_watcher(app: tauri::AppHandle) -> Result<(), String> {
    let events_path = get_agent_events_path();

    // Spawn a background task to watch the file
    tokio::spawn(async move {
        let mut last_position: u64 = 0;
        let mut last_session_id: Option<Uuid> = None;

        loop {
            // Try to open the file
            if let Ok(mut file) = File::open(&events_path) {
                // Get file size
                if let Ok(metadata) = file.metadata() {
                    let file_size = metadata.len();

                    // If file was truncated, reset position
                    if file_size < last_position {
                        last_position = 0;
                        last_session_id = None;
                    }

                    // Seek to last read position
                    if file.seek(SeekFrom::Start(last_position)).is_ok() {
                        let reader = BufReader::new(&file);

                        for line in reader.lines() {
                            if let Ok(line) = line {
                                if let Ok(event) = serde_json::from_str::<AgentEvent>(&line) {
                                    // Track session ID - only emit events from current session
                                    if last_session_id.is_none() || last_session_id == Some(event.session_id) {
                                        last_session_id = Some(event.session_id);

                                        // Emit to frontend
                                        let _ = app.emit("agent-event", &event);
                                    } else if Some(event.session_id) != last_session_id {
                                        // New session started, update filter and emit
                                        last_session_id = Some(event.session_id);
                                        // Emit session change event
                                        let _ = app.emit("agent-session-change", event.session_id.to_string());
                                        let _ = app.emit("agent-event", &event);
                                    }
                                }
                                last_position += line.len() as u64 + 1; // +1 for newline
                            }
                        }
                    }
                }
            }

            // Poll every 200ms
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    Ok(())
}

/// Get recent agent events (for initial load)
#[tauri::command]
async fn get_recent_agent_events(count: Option<usize>) -> Result<Vec<AgentEvent>, String> {
    let events_path = get_agent_events_path();
    let count = count.unwrap_or(100);

    let file = File::open(&events_path).map_err(|e| format!("Failed to open events file: {}", e))?;
    let reader = BufReader::new(file);

    let mut all_events: Vec<AgentEvent> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect();

    // Get the most recent session's events
    if let Some(last) = all_events.last() {
        let session_id = last.session_id;
        all_events.retain(|e| e.session_id == session_id);
    }

    // Return last N events
    if all_events.len() > count {
        all_events.drain(0..all_events.len() - count);
    }

    Ok(all_events)
}

// ============================================================================
// Application Entry Point
// ============================================================================

/// Run the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            health_check,
            greet,
            list_projects,
            get_project,
            list_features,
            update_feature_status,
            check_for_conflicts,
            get_conflict_details,
            resolve_conflict,
            acknowledge_edits,
            get_file_diff,
            start_agent_watcher,
            get_recent_agent_events,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            state
                .initialized
                .store(true, std::sync::atomic::Ordering::SeqCst);

            // Initialize database connection - clone the Arc
            let pool_arc = Arc::clone(&state.pool);
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
                rt.block_on(async {
                    let db_path = get_db_path();

                    if !db_path.exists() {
                        eprintln!("Database not found at {:?}", db_path);
                        return;
                    }

                    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

                    match SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect(&db_url)
                        .await
                    {
                        Ok(pool) => {
                            let mut pool_guard = pool_arc.write().await;
                            *pool_guard = Some(pool);
                            eprintln!("Database connected: {:?}", db_path);
                        }
                        Err(e) => {
                            eprintln!("Failed to connect to database: {}", e);
                        }
                    }
                });
            });

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
