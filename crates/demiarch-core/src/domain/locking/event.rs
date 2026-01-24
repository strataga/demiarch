//! Locking domain events
//!
//! Events for tracking lock-related activities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::events::DomainEvent;

/// Type of lock event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockEventType {
    /// A lock was acquired
    LockAcquired,
    /// A lock was released
    LockReleased,
    /// Lock contention occurred (another holder tried to acquire)
    LockContention,
    /// A stale lock was detected and cleaned up
    StaleLockDetected,
}

impl LockEventType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LockAcquired => "lock_acquired",
            Self::LockReleased => "lock_released",
            Self::LockContention => "lock_contention",
            Self::StaleLockDetected => "stale_lock_detected",
        }
    }
}

impl std::fmt::Display for LockEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A locking domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// The resource ID being locked
    pub aggregate_id: Uuid,
    /// Type of event
    pub event_type: LockEventType,
    /// Event data
    pub data: Option<serde_json::Value>,
    /// When the event occurred
    pub created_at: DateTime<Utc>,
}

impl LockEvent {
    /// Create a new lock event
    pub fn new(
        resource_id: Uuid,
        event_type: LockEventType,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            aggregate_id: resource_id,
            event_type,
            data,
            created_at: Utc::now(),
        }
    }

    /// Create a lock acquired event
    pub fn lock_acquired(resource_id: Uuid, holder_id: &str, resource_type: &str) -> Self {
        let data = serde_json::json!({
            "holder_id": holder_id,
            "resource_type": resource_type,
        });
        Self::new(resource_id, LockEventType::LockAcquired, Some(data))
    }

    /// Create a lock released event
    pub fn lock_released(resource_id: Uuid, holder_id: &str, duration_ms: u64) -> Self {
        let data = serde_json::json!({
            "holder_id": holder_id,
            "duration_ms": duration_ms,
        });
        Self::new(resource_id, LockEventType::LockReleased, Some(data))
    }

    /// Create a lock contention event
    pub fn lock_contention(resource_id: Uuid, current_holder: &str, blocked_holder: &str) -> Self {
        let data = serde_json::json!({
            "current_holder": current_holder,
            "blocked_holder": blocked_holder,
        });
        Self::new(resource_id, LockEventType::LockContention, Some(data))
    }

    /// Create a stale lock detected event
    pub fn stale_lock_detected(resource_id: Uuid, stale_holder: &str, age_seconds: u64) -> Self {
        let data = serde_json::json!({
            "stale_holder": stale_holder,
            "age_seconds": age_seconds,
        });
        Self::new(resource_id, LockEventType::StaleLockDetected, Some(data))
    }
}

impl DomainEvent for LockEvent {
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
    fn test_lock_acquired_event() {
        let resource_id = Uuid::new_v4();
        let event = LockEvent::lock_acquired(resource_id, "session-123", "file");

        assert_eq!(event.aggregate_id, resource_id);
        assert_eq!(event.event_type, LockEventType::LockAcquired);

        let data = event.data.unwrap();
        assert_eq!(data["holder_id"], "session-123");
        assert_eq!(data["resource_type"], "file");
    }

    #[test]
    fn test_lock_released_event() {
        let resource_id = Uuid::new_v4();
        let event = LockEvent::lock_released(resource_id, "session-123", 5000);

        assert_eq!(event.event_type, LockEventType::LockReleased);

        let data = event.data.unwrap();
        assert_eq!(data["duration_ms"], 5000);
    }

    #[test]
    fn test_lock_contention_event() {
        let resource_id = Uuid::new_v4();
        let event = LockEvent::lock_contention(resource_id, "holder-1", "holder-2");

        assert_eq!(event.event_type, LockEventType::LockContention);

        let data = event.data.unwrap();
        assert_eq!(data["current_holder"], "holder-1");
        assert_eq!(data["blocked_holder"], "holder-2");
    }

    #[test]
    fn test_stale_lock_event() {
        let resource_id = Uuid::new_v4();
        let event = LockEvent::stale_lock_detected(resource_id, "dead-holder", 3600);

        assert_eq!(event.event_type, LockEventType::StaleLockDetected);

        let data = event.data.unwrap();
        assert_eq!(data["stale_holder"], "dead-holder");
        assert_eq!(data["age_seconds"], 3600);
    }

    #[test]
    fn test_domain_event_impl() {
        let event = LockEvent::lock_acquired(Uuid::new_v4(), "holder", "resource");
        assert_eq!(event.event_type(), "lock_acquired");
    }
}
