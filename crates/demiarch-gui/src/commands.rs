//! Tauri command handlers
//!
//! These commands bridge the React frontend to demiarch-core functionality.
//! Each command is exposed to the frontend via Tauri's invoke system.

use demiarch_core::api;
use serde::{Deserialize, Serialize};

/// Project summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub status: String,
    pub feature_count: usize,
}

impl From<api::projects::ProjectSummary> for ProjectSummary {
    fn from(p: api::projects::ProjectSummary) -> Self {
        Self {
            id: p.id,
            name: p.name,
            framework: p.framework,
            status: p.status,
            feature_count: p.feature_count,
        }
    }
}

/// Feature summary for Kanban board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i32,
    pub phase_id: String,
}

impl From<api::features::FeatureSummary> for FeatureSummary {
    fn from(f: api::features::FeatureSummary) -> Self {
        Self {
            id: f.id,
            name: f.title,
            description: f.description,
            status: f.status,
            priority: f.priority,
            phase_id: f.phase_id.unwrap_or_default(),
        }
    }
}

/// Agent status for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub id: String,
    pub agent_type: String,
    pub status: String,
    pub parent_id: Option<String>,
    pub task: Option<String>,
    pub tokens_used: u64,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub today_usd: f64,
    pub daily_limit_usd: f64,
    pub remaining_usd: f64,
    pub alert_threshold: f64,
}

/// Session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub status: String,
    pub current_project_id: Option<String>,
    pub started_at: String,
}

impl From<api::sessions::SessionSummary> for SessionSummary {
    fn from(s: api::sessions::SessionSummary) -> Self {
        Self {
            id: s.id,
            status: s.status,
            current_project_id: s.current_project_id,
            started_at: s.created_at,
        }
    }
}

/// Doctor check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorResult {
    pub config_ok: bool,
    pub api_key_ok: bool,
    pub database_ok: bool,
    pub schema_version: i32,
    pub project_count: usize,
}

/// Conflict hunk for resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub id: String,
    pub start_line: u32,
    pub end_line: u32,
    pub user_content: String,
    pub ai_content: String,
    pub resolved: bool,
    pub resolution: Option<String>,
    pub custom_content: Option<String>,
}

/// Conflict for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub file_path: String,
    pub hunks: Vec<ConflictHunk>,
    pub created_at: String,
}

// ============================================================
// Project Commands
// ============================================================

#[tauri::command]
pub async fn get_projects() -> Result<Vec<ProjectSummary>, String> {
    let projects = api::projects::list(None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(projects.into_iter().map(ProjectSummary::from).collect())
}

#[tauri::command]
pub async fn get_project(id: String) -> Result<ProjectSummary, String> {
    let project = api::projects::get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", id))?;
    Ok(ProjectSummary::from(project))
}

#[tauri::command]
pub async fn create_project(name: String, framework: String) -> Result<ProjectSummary, String> {
    let request = api::projects::CreateProjectRequest {
        name,
        framework,
        repo_url: None,
        description: None,
        path: None,
    };
    let project = api::projects::create(request)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ProjectSummary::from(project))
}

// ============================================================
// Feature Commands
// ============================================================

#[tauri::command]
pub async fn get_features(project_id: String) -> Result<Vec<FeatureSummary>, String> {
    let features = api::features::list_by_project(&project_id, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(features.into_iter().map(FeatureSummary::from).collect())
}

#[tauri::command]
pub async fn get_feature(id: String) -> Result<FeatureSummary, String> {
    let feature = api::features::get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Feature not found: {}", id))?;
    Ok(FeatureSummary::from(feature))
}

#[tauri::command]
pub async fn update_feature_status(id: String, status: String) -> Result<FeatureSummary, String> {
    api::features::update_status(&id, &status)
        .await
        .map_err(|e| e.to_string())?;

    // Return the updated feature
    let feature = api::features::get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Feature not found: {}", id))?;
    Ok(FeatureSummary::from(feature))
}

// ============================================================
// Session Commands
// ============================================================

#[tauri::command]
pub async fn get_sessions() -> Result<Vec<SessionSummary>, String> {
    let sessions = api::sessions::list(None, Some(50))
        .await
        .map_err(|e| e.to_string())?;
    Ok(sessions.into_iter().map(SessionSummary::from).collect())
}

// ============================================================
// Cost Commands
// ============================================================

#[tauri::command]
pub async fn get_costs() -> Result<CostSummary, String> {
    // Cost tracking requires a CostTracker instance which is typically
    // managed by the application state. For now, return defaults.
    // TODO: Add CostTracker to application state
    Ok(CostSummary {
        today_usd: 0.0,
        daily_limit_usd: 10.0,
        remaining_usd: 10.0,
        alert_threshold: 0.8,
    })
}

// ============================================================
// Agent Commands
// ============================================================

#[tauri::command]
pub async fn get_agents() -> Result<Vec<AgentStatus>, String> {
    // Agent status tracking is not yet implemented in the core API
    // TODO: Add agent status API
    Ok(vec![])
}

// ============================================================
// System Commands
// ============================================================

#[tauri::command]
pub async fn doctor() -> Result<DoctorResult, String> {
    let health = api::health::doctor()
        .await
        .map_err(|e| e.to_string())?;

    let database_ok = health.checks.iter()
        .find(|c| c.name == "Database")
        .map(|c| c.status == api::health::HealthStatus::Ok)
        .unwrap_or(false);

    let config_ok = health.checks.iter()
        .find(|c| c.name == "Configuration")
        .map(|c| c.status != api::health::HealthStatus::Error)
        .unwrap_or(false);

    // Get project count
    let project_count = api::projects::list(None)
        .await
        .map(|p| p.len())
        .unwrap_or(0);

    Ok(DoctorResult {
        config_ok,
        api_key_ok: true, // TODO: Check API key
        database_ok,
        schema_version: 11, // TODO: Get from migrations
        project_count,
    })
}

// ============================================================
// Conflict Resolution Commands
// ============================================================

#[tauri::command]
pub async fn get_conflicts(project_id: String) -> Result<Vec<Conflict>, String> {
    // Conflict resolution is not yet implemented in the core API
    // TODO: Add conflict resolution API
    let _ = project_id;
    Ok(vec![])
}

#[tauri::command]
pub async fn resolve_conflict_hunk(
    conflict_id: String,
    hunk_id: String,
    resolution: String,
    custom_content: Option<String>,
) -> Result<(), String> {
    // Conflict resolution is not yet implemented in the core API
    // TODO: Add conflict resolution API
    let _ = (conflict_id, hunk_id, resolution, custom_content);
    Ok(())
}

#[tauri::command]
pub async fn apply_conflict_resolutions(conflict_id: String) -> Result<(), String> {
    // Conflict resolution is not yet implemented in the core API
    // TODO: Add conflict resolution API
    let _ = conflict_id;
    Ok(())
}
