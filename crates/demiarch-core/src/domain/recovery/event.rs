//! Recovery domain events
//!
//! Events for tracking checkpoint and recovery activities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::events::DomainEvent;

/// Type of recovery event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryEventType {
    /// A checkpoint was created
    CheckpointCreated,
    /// A checkpoint was restored
    CheckpointRestored,
    /// An external edit was detected
    EditDetected,
    /// A checkpoint expired and was cleaned up
    CheckpointExpired,
}

impl RecoveryEventType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CheckpointCreated => "checkpoint_created",
            Self::CheckpointRestored => "checkpoint_restored",
            Self::EditDetected => "edit_detected",
            Self::CheckpointExpired => "checkpoint_expired",
        }
    }
}

impl std::fmt::Display for RecoveryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A recovery domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// The checkpoint or session ID
    pub aggregate_id: Uuid,
    /// Type of event
    pub event_type: RecoveryEventType,
    /// Event data
    pub data: Option<serde_json::Value>,
    /// When the event occurred
    pub created_at: DateTime<Utc>,
}

impl RecoveryEvent {
    /// Create a new recovery event
    pub fn new(
        aggregate_id: Uuid,
        event_type: RecoveryEventType,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            aggregate_id,
            event_type,
            data,
            created_at: Utc::now(),
        }
    }

    /// Create a checkpoint created event
    pub fn checkpoint_created(
        checkpoint_id: Uuid,
        session_id: Uuid,
        file_count: usize,
        total_size_bytes: u64,
    ) -> Self {
        let data = serde_json::json!({
            "session_id": session_id,
            "file_count": file_count,
            "total_size_bytes": total_size_bytes,
        });
        Self::new(checkpoint_id, RecoveryEventType::CheckpointCreated, Some(data))
    }

    /// Create a checkpoint restored event
    pub fn checkpoint_restored(
        checkpoint_id: Uuid,
        session_id: Uuid,
        files_restored: usize,
    ) -> Self {
        let data = serde_json::json!({
            "session_id": session_id,
            "files_restored": files_restored,
        });
        Self::new(checkpoint_id, RecoveryEventType::CheckpointRestored, Some(data))
    }

    /// Create an edit detected event
    pub fn edit_detected(
        session_id: Uuid,
        file_path: &str,
        edit_type: &str,
    ) -> Self {
        let data = serde_json::json!({
            "file_path": file_path,
            "edit_type": edit_type,
        });
        Self::new(session_id, RecoveryEventType::EditDetected, Some(data))
    }

    /// Create a checkpoint expired event
    pub fn checkpoint_expired(
        checkpoint_id: Uuid,
        age_seconds: u64,
        reason: &str,
    ) -> Self {
        let data = serde_json::json!({
            "age_seconds": age_seconds,
            "reason": reason,
        });
        Self::new(checkpoint_id, RecoveryEventType::CheckpointExpired, Some(data))
    }
}

impl DomainEvent for RecoveryEvent {
    fn event_type(&self) -> &str {
        self.event_type.as_str()
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn data(&self) -> Option<&serde_json::Value> {
        self.data.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_created_event() {
        let checkpoint_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let event = RecoveryEvent::checkpoint_created(checkpoint_id, session_id, 10, 1024 * 1024);

        assert_eq!(event.aggregate_id, checkpoint_id);
        assert_eq!(event.event_type, RecoveryEventType::CheckpointCreated);

        let data = event.data.unwrap();
        assert_eq!(data["file_count"], 10);
        assert_eq!(data["total_size_bytes"], 1024 * 1024);
    }

    #[test]
    fn test_checkpoint_restored_event() {
        let checkpoint_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let event = RecoveryEvent::checkpoint_restored(checkpoint_id, session_id, 8);

        assert_eq!(event.event_type, RecoveryEventType::CheckpointRestored);

        let data = event.data.unwrap();
        assert_eq!(data["files_restored"], 8);
    }

    #[test]
    fn test_edit_detected_event() {
        let session_id = Uuid::new_v4();
        let event = RecoveryEvent::edit_detected(session_id, "/src/main.rs", "modified");

        assert_eq!(event.event_type, RecoveryEventType::EditDetected);

        let data = event.data.unwrap();
        assert_eq!(data["file_path"], "/src/main.rs");
        assert_eq!(data["edit_type"], "modified");
    }

    #[test]
    fn test_checkpoint_expired_event() {
        let checkpoint_id = Uuid::new_v4();
        let event = RecoveryEvent::checkpoint_expired(checkpoint_id, 86400, "max age exceeded");

        assert_eq!(event.event_type, RecoveryEventType::CheckpointExpired);

        let data = event.data.unwrap();
        assert_eq!(data["age_seconds"], 86400);
        assert_eq!(data["reason"], "max age exceeded");
    }

    #[test]
    fn test_domain_event_impl() {
        let event = RecoveryEvent::checkpoint_created(Uuid::new_v4(), Uuid::new_v4(), 1, 100);
        assert_eq!(event.event_type(), "checkpoint_created");
    }
}
