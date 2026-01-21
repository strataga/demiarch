//! Session entity and related types
//!
//! Defines the core Session type and its associated statuses and phases.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Session status indicating the current state of a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session is active and being used
    Active,
    /// Session is temporarily paused
    Paused,
    /// Session has been completed successfully
    Completed,
    /// Session was abandoned without completion
    Abandoned,
}

impl SessionStatus {
    /// Create from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "abandoned" => Some(Self::Abandoned),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    /// Check if session is still ongoing (active or paused)
    pub fn is_ongoing(&self) -> bool {
        matches!(self, Self::Active | Self::Paused)
    }

    /// Check if session has ended (completed or abandoned)
    pub fn has_ended(&self) -> bool {
        matches!(self, Self::Completed | Self::Abandoned)
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Session phase indicating the current workflow phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    /// Discovery and requirements gathering
    Discovery,
    /// Technical planning and architecture
    Planning,
    /// Implementation and development
    Building,
    /// Testing and validation
    Testing,
    /// Review and refinement
    Review,
    /// Phase not yet determined
    Unknown,
}

impl SessionPhase {
    /// Create from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "discovery" => Some(Self::Discovery),
            "planning" => Some(Self::Planning),
            "building" => Some(Self::Building),
            "testing" => Some(Self::Testing),
            "review" => Some(Self::Review),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Planning => "planning",
            Self::Building => "building",
            Self::Testing => "testing",
            Self::Review => "review",
            Self::Unknown => "unknown",
        }
    }
}

impl Default for SessionPhase {
    fn default() -> Self {
        Self::Unknown
    }
}

impl fmt::Display for SessionPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A global session tracking work across multiple projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: Uuid,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// When the session was last updated
    pub updated_at: DateTime<Utc>,

    /// When the session was last active
    pub last_activity: DateTime<Utc>,

    /// Current project being worked on (if any)
    pub current_project_id: Option<Uuid>,

    /// Current feature being worked on (if any)
    pub current_feature_id: Option<Uuid>,

    /// Current session status
    pub status: SessionStatus,

    /// Current workflow phase
    pub phase: SessionPhase,

    /// Human-readable description of what's being worked on
    pub description: Option<String>,

    /// Last checkpoint ID for recovery
    pub last_checkpoint_id: Option<Uuid>,

    /// Metadata for extensibility (JSON)
    pub metadata: Option<serde_json::Value>,
}

impl Session {
    /// Create a new active session
    pub fn new(
        current_project_id: Option<Uuid>,
        current_feature_id: Option<Uuid>,
        description: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            last_activity: now,
            current_project_id,
            current_feature_id,
            status: SessionStatus::Active,
            phase: SessionPhase::Unknown,
            description,
            last_checkpoint_id: None,
            metadata: None,
        }
    }

    /// Create a new session with a specific ID (for testing or recovery)
    pub fn with_id(
        id: Uuid,
        current_project_id: Option<Uuid>,
        current_feature_id: Option<Uuid>,
        description: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            created_at: now,
            updated_at: now,
            last_activity: now,
            current_project_id,
            current_feature_id,
            status: SessionStatus::Active,
            phase: SessionPhase::Unknown,
            description,
            last_checkpoint_id: None,
            metadata: None,
        }
    }

    /// Update the last activity timestamp
    pub fn touch(&mut self) {
        let now = Utc::now();
        self.last_activity = now;
        self.updated_at = now;
    }

    /// Set the current project
    pub fn set_project(&mut self, project_id: Option<Uuid>) {
        self.current_project_id = project_id;
        self.touch();
    }

    /// Set the current feature
    pub fn set_feature(&mut self, feature_id: Option<Uuid>) {
        self.current_feature_id = feature_id;
        self.touch();
    }

    /// Set the session phase
    pub fn set_phase(&mut self, phase: SessionPhase) {
        self.phase = phase;
        self.touch();
    }

    /// Pause the session
    pub fn pause(&mut self) {
        self.status = SessionStatus::Paused;
        self.touch();
    }

    /// Resume the session
    pub fn resume(&mut self) {
        self.status = SessionStatus::Active;
        self.touch();
    }

    /// Complete the session
    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.touch();
    }

    /// Abandon the session
    pub fn abandon(&mut self) {
        self.status = SessionStatus::Abandoned;
        self.touch();
    }

    /// Set the last checkpoint ID
    pub fn set_checkpoint(&mut self, checkpoint_id: Uuid) {
        self.last_checkpoint_id = Some(checkpoint_id);
        self.touch();
    }

    /// Get the duration of the session
    pub fn duration(&self) -> chrono::Duration {
        let end_time = if self.status.has_ended() {
            self.updated_at
        } else {
            Utc::now()
        };
        end_time - self.created_at
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.status == SessionStatus::Active
    }

    /// Check if session is paused
    pub fn is_paused(&self) -> bool {
        self.status == SessionStatus::Paused
    }
}

/// Lightweight session info for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub id: Uuid,

    /// Current session status
    pub status: SessionStatus,

    /// Current workflow phase
    pub phase: SessionPhase,

    /// Current project ID
    pub current_project_id: Option<Uuid>,

    /// Human-readable description
    pub description: Option<String>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id,
            status: session.status,
            phase: session.phase,
            current_project_id: session.current_project_id,
            description: session.description.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_from_str() {
        assert_eq!(SessionStatus::from_str("active"), Some(SessionStatus::Active));
        assert_eq!(SessionStatus::from_str("PAUSED"), Some(SessionStatus::Paused));
        assert_eq!(SessionStatus::from_str("Completed"), Some(SessionStatus::Completed));
        assert_eq!(SessionStatus::from_str("abandoned"), Some(SessionStatus::Abandoned));
        assert_eq!(SessionStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_session_status_is_ongoing() {
        assert!(SessionStatus::Active.is_ongoing());
        assert!(SessionStatus::Paused.is_ongoing());
        assert!(!SessionStatus::Completed.is_ongoing());
        assert!(!SessionStatus::Abandoned.is_ongoing());
    }

    #[test]
    fn test_session_phase_from_str() {
        assert_eq!(SessionPhase::from_str("discovery"), Some(SessionPhase::Discovery));
        assert_eq!(SessionPhase::from_str("BUILDING"), Some(SessionPhase::Building));
        assert_eq!(SessionPhase::from_str("invalid"), None);
    }

    #[test]
    fn test_session_creation() {
        let project_id = Uuid::new_v4();
        let session = Session::new(Some(project_id), None, Some("Test session".to_string()));

        assert!(session.is_active());
        assert_eq!(session.current_project_id, Some(project_id));
        assert_eq!(session.phase, SessionPhase::Unknown);
        assert_eq!(session.description, Some("Test session".to_string()));
    }

    #[test]
    fn test_session_lifecycle() {
        let mut session = Session::new(None, None, None);

        // Start active
        assert!(session.is_active());

        // Pause
        session.pause();
        assert!(session.is_paused());
        assert!(session.status.is_ongoing());

        // Resume
        session.resume();
        assert!(session.is_active());

        // Complete
        session.complete();
        assert!(session.status.has_ended());
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_session_project_switching() {
        let project1 = Uuid::new_v4();
        let project2 = Uuid::new_v4();
        let mut session = Session::new(Some(project1), None, None);

        assert_eq!(session.current_project_id, Some(project1));

        session.set_project(Some(project2));
        assert_eq!(session.current_project_id, Some(project2));

        session.set_project(None);
        assert_eq!(session.current_project_id, None);
    }

    #[test]
    fn test_session_info_from_session() {
        let session = Session::new(None, None, Some("Test".to_string()));
        let info: SessionInfo = (&session).into();

        assert_eq!(info.id, session.id);
        assert_eq!(info.status, session.status);
        assert_eq!(info.description, session.description);
    }
}
