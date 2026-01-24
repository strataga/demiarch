//! Sessions API
//!
//! Provides high-level operations for session management from GUI.

use crate::domain::session::{Session, SessionInfo, SessionPhase, SessionRepository, SessionStatus};
use crate::Result;
use serde::{Deserialize, Serialize};

use super::get_database;

/// Session summary for GUI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub status: String,
    pub phase: String,
    pub current_project_id: Option<String>,
    pub current_feature_id: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity: String,
    pub has_checkpoint: bool,
}

impl From<Session> for SessionSummary {
    fn from(s: Session) -> Self {
        Self {
            id: s.id.to_string(),
            status: s.status.as_str().to_string(),
            phase: s.phase.as_str().to_string(),
            current_project_id: s.current_project_id.map(|id| id.to_string()),
            current_feature_id: s.current_feature_id.map(|id| id.to_string()),
            description: s.description,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
            last_activity: s.last_activity.to_rfc3339(),
            has_checkpoint: s.last_checkpoint_id.is_some(),
        }
    }
}

impl From<SessionInfo> for SessionSummary {
    fn from(s: SessionInfo) -> Self {
        Self {
            id: s.id.to_string(),
            status: s.status.as_str().to_string(),
            phase: s.phase.as_str().to_string(),
            current_project_id: s.current_project_id.map(|id| id.to_string()),
            current_feature_id: None, // SessionInfo doesn't have feature_id
            description: s.description,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.created_at.to_rfc3339(), // Use created_at as fallback
            last_activity: s.last_activity.to_rfc3339(),
            has_checkpoint: false, // SessionInfo doesn't have checkpoint info
        }
    }
}

/// List all sessions
pub async fn list(status: Option<&str>, limit: Option<i32>) -> Result<Vec<SessionSummary>> {
    let db = get_database().await?;
    let repo = SessionRepository::new(db.pool().clone());

    let sessions = if let Some(status_str) = status {
        let status_filter = SessionStatus::parse(status_str)
            .ok_or_else(|| crate::Error::InvalidInput(format!("Invalid status: {}", status_str)))?;
        repo.list_by_status(status_filter).await?
    } else {
        repo.list(limit).await?
    };

    Ok(sessions.into_iter().map(SessionSummary::from).collect())
}

/// Get current active session (if any)
pub async fn get_active() -> Result<Option<SessionSummary>> {
    let db = get_database().await?;
    let repo = SessionRepository::new(db.pool().clone());

    let session = repo.get_active().await?;
    Ok(session.map(SessionSummary::from))
}

/// Get a single session by ID
pub async fn get(id: &str) -> Result<Option<SessionSummary>> {
    let db = get_database().await?;
    let repo = SessionRepository::new(db.pool().clone());

    let uuid = uuid::Uuid::parse_str(id)
        .map_err(|_| crate::Error::InvalidInput(format!("Invalid session ID: {}", id)))?;

    let session = repo.get(uuid).await?;
    Ok(session.map(SessionSummary::from))
}

/// Create a new session
pub async fn create(
    project_id: Option<&str>,
    feature_id: Option<&str>,
    description: Option<&str>,
) -> Result<SessionSummary> {
    let db = get_database().await?;
    let repo = SessionRepository::new(db.pool().clone());

    let project_uuid = project_id
        .map(uuid::Uuid::parse_str)
        .transpose()
        .map_err(|_| crate::Error::InvalidInput("Invalid project ID".to_string()))?;

    let feature_uuid = feature_id
        .map(uuid::Uuid::parse_str)
        .transpose()
        .map_err(|_| crate::Error::InvalidInput("Invalid feature ID".to_string()))?;

    let session = Session::new(project_uuid, feature_uuid, description.map(String::from));
    repo.save(&session).await?;

    Ok(SessionSummary::from(session))
}

/// Update session status
pub async fn update_status(id: &str, status: &str) -> Result<()> {
    let db = get_database().await?;
    let repo = SessionRepository::new(db.pool().clone());

    let uuid = uuid::Uuid::parse_str(id)
        .map_err(|_| crate::Error::InvalidInput(format!("Invalid session ID: {}", id)))?;

    let mut session = repo
        .get(uuid)
        .await?
        .ok_or_else(|| crate::Error::NotFound(format!("Session not found: {}", id)))?;

    match status {
        "active" => session.resume(),
        "paused" => session.pause(),
        "completed" => session.complete(),
        "abandoned" => session.abandon(),
        _ => {
            return Err(crate::Error::InvalidInput(format!(
                "Invalid status: {}",
                status
            )))
        }
    }

    repo.update(&session).await?;
    Ok(())
}

/// Update session phase
pub async fn update_phase(id: &str, phase: &str) -> Result<()> {
    let db = get_database().await?;
    let repo = SessionRepository::new(db.pool().clone());

    let uuid = uuid::Uuid::parse_str(id)
        .map_err(|_| crate::Error::InvalidInput(format!("Invalid session ID: {}", id)))?;

    let mut session = repo
        .get(uuid)
        .await?
        .ok_or_else(|| crate::Error::NotFound(format!("Session not found: {}", id)))?;

    let new_phase = SessionPhase::parse(phase)
        .ok_or_else(|| crate::Error::InvalidInput(format!("Invalid phase: {}", phase)))?;

    session.set_phase(new_phase);
    repo.update(&session).await?;
    Ok(())
}
