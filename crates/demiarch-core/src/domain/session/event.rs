//! Session event types for tracking session activities
//!
//! Events provide an audit trail of session activities for debugging and recovery.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of session event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    /// Session was created/started
    Started,
    /// Session was paused
    Paused,
    /// Session was resumed
    Resumed,
    /// Session was completed
    Completed,
    /// Session was abandoned
    Abandoned,
    /// Project was switched
    ProjectSwitched,
    /// Feature was switched
    FeatureSwitched,
    /// Phase was changed
    PhaseChanged,
    /// Checkpoint was created
    CheckpointCreated,
    /// An error occurred
    Error,
    /// Custom event
    Custom,
}

impl SessionEventType {
    /// Create from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "started" => Some(Self::Started),
            "paused" => Some(Self::Paused),
            "resumed" => Some(Self::Resumed),
            "completed" => Some(Self::Completed),
            "abandoned" => Some(Self::Abandoned),
            "project_switched" => Some(Self::ProjectSwitched),
            "feature_switched" => Some(Self::FeatureSwitched),
            "phase_changed" => Some(Self::PhaseChanged),
            "checkpoint_created" => Some(Self::CheckpointCreated),
            "error" => Some(Self::Error),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Paused => "paused",
            Self::Resumed => "resumed",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
            Self::ProjectSwitched => "project_switched",
            Self::FeatureSwitched => "feature_switched",
            Self::PhaseChanged => "phase_changed",
            Self::CheckpointCreated => "checkpoint_created",
            Self::Error => "error",
            Self::Custom => "custom",
        }
    }
}

impl std::fmt::Display for SessionEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A session event representing an action or state change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    /// Unique event identifier
    pub id: Uuid,

    /// Session this event belongs to
    pub session_id: Uuid,

    /// Type of event
    pub event_type: SessionEventType,

    /// Event data (JSON)
    pub data: Option<serde_json::Value>,

    /// When the event occurred
    pub created_at: DateTime<Utc>,
}

impl SessionEvent {
    /// Create a new session event
    pub fn new(session_id: Uuid, event_type: SessionEventType, data: Option<serde_json::Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            event_type,
            data,
            created_at: Utc::now(),
        }
    }

    /// Create a started event
    pub fn started(session_id: Uuid) -> Self {
        Self::new(session_id, SessionEventType::Started, None)
    }

    /// Create a paused event
    pub fn paused(session_id: Uuid) -> Self {
        Self::new(session_id, SessionEventType::Paused, None)
    }

    /// Create a resumed event
    pub fn resumed(session_id: Uuid) -> Self {
        Self::new(session_id, SessionEventType::Resumed, None)
    }

    /// Create a completed event
    pub fn completed(session_id: Uuid) -> Self {
        Self::new(session_id, SessionEventType::Completed, None)
    }

    /// Create an abandoned event
    pub fn abandoned(session_id: Uuid) -> Self {
        Self::new(session_id, SessionEventType::Abandoned, None)
    }

    /// Create a project switched event
    pub fn project_switched(session_id: Uuid, old_project_id: Option<Uuid>, new_project_id: Option<Uuid>) -> Self {
        let data = serde_json::json!({
            "old_project_id": old_project_id,
            "new_project_id": new_project_id,
        });
        Self::new(session_id, SessionEventType::ProjectSwitched, Some(data))
    }

    /// Create a feature switched event
    pub fn feature_switched(session_id: Uuid, old_feature_id: Option<Uuid>, new_feature_id: Option<Uuid>) -> Self {
        let data = serde_json::json!({
            "old_feature_id": old_feature_id,
            "new_feature_id": new_feature_id,
        });
        Self::new(session_id, SessionEventType::FeatureSwitched, Some(data))
    }

    /// Create a phase changed event
    pub fn phase_changed(session_id: Uuid, old_phase: &str, new_phase: &str) -> Self {
        let data = serde_json::json!({
            "old_phase": old_phase,
            "new_phase": new_phase,
        });
        Self::new(session_id, SessionEventType::PhaseChanged, Some(data))
    }

    /// Create a checkpoint created event
    pub fn checkpoint_created(session_id: Uuid, checkpoint_id: Uuid) -> Self {
        let data = serde_json::json!({
            "checkpoint_id": checkpoint_id,
        });
        Self::new(session_id, SessionEventType::CheckpointCreated, Some(data))
    }

    /// Create an error event
    pub fn error(session_id: Uuid, error_message: &str, error_code: Option<&str>) -> Self {
        let data = serde_json::json!({
            "message": error_message,
            "code": error_code,
        });
        Self::new(session_id, SessionEventType::Error, Some(data))
    }

    /// Create a custom event
    pub fn custom(session_id: Uuid, event_name: &str, data: Option<serde_json::Value>) -> Self {
        let event_data = serde_json::json!({
            "name": event_name,
            "data": data,
        });
        Self::new(session_id, SessionEventType::Custom, Some(event_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(SessionEventType::from_str("started"), Some(SessionEventType::Started));
        assert_eq!(SessionEventType::from_str("PAUSED"), Some(SessionEventType::Paused));
        assert_eq!(SessionEventType::from_str("project_switched"), Some(SessionEventType::ProjectSwitched));
        assert_eq!(SessionEventType::from_str("invalid"), None);
    }

    #[test]
    fn test_event_creation() {
        let session_id = Uuid::new_v4();
        let event = SessionEvent::started(session_id);

        assert_eq!(event.session_id, session_id);
        assert_eq!(event.event_type, SessionEventType::Started);
        assert!(event.data.is_none());
    }

    #[test]
    fn test_project_switched_event() {
        let session_id = Uuid::new_v4();
        let old_project = Uuid::new_v4();
        let new_project = Uuid::new_v4();

        let event = SessionEvent::project_switched(session_id, Some(old_project), Some(new_project));

        assert_eq!(event.event_type, SessionEventType::ProjectSwitched);
        assert!(event.data.is_some());

        let data = event.data.unwrap();
        assert_eq!(data["old_project_id"], old_project.to_string());
        assert_eq!(data["new_project_id"], new_project.to_string());
    }

    #[test]
    fn test_error_event() {
        let session_id = Uuid::new_v4();
        let event = SessionEvent::error(session_id, "Something went wrong", Some("E001"));

        assert_eq!(event.event_type, SessionEventType::Error);
        let data = event.data.unwrap();
        assert_eq!(data["message"], "Something went wrong");
        assert_eq!(data["code"], "E001");
    }
}
