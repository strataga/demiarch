//! Security domain events
//!
//! Events for tracking key management activities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::events::DomainEvent;

/// Type of security event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    /// A key was stored
    KeyStored,
    /// A key was retrieved
    KeyRetrieved,
    /// A key was rotated
    KeyRotated,
    /// Decryption failed
    DecryptionFailed,
}

impl SecurityEventType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KeyStored => "key_stored",
            Self::KeyRetrieved => "key_retrieved",
            Self::KeyRotated => "key_rotated",
            Self::DecryptionFailed => "decryption_failed",
        }
    }
}

impl std::fmt::Display for SecurityEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A security domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// The key ID this event relates to
    pub aggregate_id: Uuid,
    /// Type of event
    pub event_type: SecurityEventType,
    /// Event data (no sensitive data!)
    pub data: Option<serde_json::Value>,
    /// When the event occurred
    pub created_at: DateTime<Utc>,
}

impl SecurityEvent {
    /// Create a new security event
    pub fn new(
        key_id: Uuid,
        event_type: SecurityEventType,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            aggregate_id: key_id,
            event_type,
            data,
            created_at: Utc::now(),
        }
    }

    /// Create a key stored event
    ///
    /// Note: Never log the actual key value!
    pub fn key_stored(key_id: Uuid, key_name: &str, algorithm: &str) -> Self {
        let data = serde_json::json!({
            "key_name": key_name,
            "algorithm": algorithm,
        });
        Self::new(key_id, SecurityEventType::KeyStored, Some(data))
    }

    /// Create a key retrieved event
    pub fn key_retrieved(key_id: Uuid, key_name: &str, requester: Option<&str>) -> Self {
        let data = serde_json::json!({
            "key_name": key_name,
            "requester": requester,
        });
        Self::new(key_id, SecurityEventType::KeyRetrieved, Some(data))
    }

    /// Create a key rotated event
    pub fn key_rotated(key_id: Uuid, key_name: &str, reason: &str) -> Self {
        let data = serde_json::json!({
            "key_name": key_name,
            "reason": reason,
        });
        Self::new(key_id, SecurityEventType::KeyRotated, Some(data))
    }

    /// Create a decryption failed event
    pub fn decryption_failed(key_id: Uuid, key_name: &str, error_code: &str) -> Self {
        let data = serde_json::json!({
            "key_name": key_name,
            "error_code": error_code,
        });
        Self::new(key_id, SecurityEventType::DecryptionFailed, Some(data))
    }
}

impl DomainEvent for SecurityEvent {
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
    fn test_key_stored_event() {
        let key_id = Uuid::new_v4();
        let event = SecurityEvent::key_stored(key_id, "api_key", "AES-256-GCM");

        assert_eq!(event.aggregate_id, key_id);
        assert_eq!(event.event_type, SecurityEventType::KeyStored);

        let data = event.data.unwrap();
        assert_eq!(data["key_name"], "api_key");
        assert_eq!(data["algorithm"], "AES-256-GCM");
    }

    #[test]
    fn test_key_retrieved_event() {
        let key_id = Uuid::new_v4();
        let event = SecurityEvent::key_retrieved(key_id, "api_key", Some("llm_client"));

        assert_eq!(event.event_type, SecurityEventType::KeyRetrieved);

        let data = event.data.unwrap();
        assert_eq!(data["requester"], "llm_client");
    }

    #[test]
    fn test_key_rotated_event() {
        let key_id = Uuid::new_v4();
        let event = SecurityEvent::key_rotated(key_id, "master_key", "scheduled rotation");

        assert_eq!(event.event_type, SecurityEventType::KeyRotated);

        let data = event.data.unwrap();
        assert_eq!(data["reason"], "scheduled rotation");
    }

    #[test]
    fn test_decryption_failed_event() {
        let key_id = Uuid::new_v4();
        let event = SecurityEvent::decryption_failed(key_id, "api_key", "INVALID_TAG");

        assert_eq!(event.event_type, SecurityEventType::DecryptionFailed);

        let data = event.data.unwrap();
        assert_eq!(data["error_code"], "INVALID_TAG");
    }

    #[test]
    fn test_domain_event_impl() {
        let event = SecurityEvent::key_stored(Uuid::new_v4(), "key", "AES");
        assert_eq!(event.event_type(), "key_stored");
    }
}
