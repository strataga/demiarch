//! Search domain events
//!
//! Events for tracking search-related activities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::events::DomainEvent;

/// Type of search event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchEventType {
    /// A cross-project search was executed
    CrossProjectSearchExecuted,
    /// Search settings were changed
    SearchSettingsChanged,
    /// Access to search was denied
    SearchAccessDenied,
}

impl SearchEventType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CrossProjectSearchExecuted => "cross_project_search_executed",
            Self::SearchSettingsChanged => "search_settings_changed",
            Self::SearchAccessDenied => "search_access_denied",
        }
    }
}

impl std::fmt::Display for SearchEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A search domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEvent {
    /// Unique event identifier
    pub id: Uuid,
    /// The aggregate ID (could be search ID, settings ID, or user ID)
    pub aggregate_id: Uuid,
    /// Type of event
    pub event_type: SearchEventType,
    /// Event data
    pub data: Option<serde_json::Value>,
    /// When the event occurred
    pub created_at: DateTime<Utc>,
}

impl SearchEvent {
    /// Create a new search event
    pub fn new(
        aggregate_id: Uuid,
        event_type: SearchEventType,
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

    /// Create a cross-project search executed event
    pub fn cross_project_search_executed(
        search_id: Uuid,
        query: &str,
        project_count: usize,
        result_count: usize,
    ) -> Self {
        let data = serde_json::json!({
            "query": query,
            "project_count": project_count,
            "result_count": result_count,
        });
        Self::new(search_id, SearchEventType::CrossProjectSearchExecuted, Some(data))
    }

    /// Create a search settings changed event
    pub fn settings_changed(
        settings_id: Uuid,
        field_name: &str,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
    ) -> Self {
        let data = serde_json::json!({
            "field": field_name,
            "old_value": old_value,
            "new_value": new_value,
        });
        Self::new(settings_id, SearchEventType::SearchSettingsChanged, Some(data))
    }

    /// Create a search access denied event
    pub fn access_denied(
        user_id: Uuid,
        resource_id: Uuid,
        reason: &str,
    ) -> Self {
        let data = serde_json::json!({
            "resource_id": resource_id,
            "reason": reason,
        });
        Self::new(user_id, SearchEventType::SearchAccessDenied, Some(data))
    }
}

impl DomainEvent for SearchEvent {
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
    fn test_search_event_creation() {
        let search_id = Uuid::new_v4();
        let event = SearchEvent::cross_project_search_executed(search_id, "test query", 5, 42);

        assert_eq!(event.aggregate_id, search_id);
        assert_eq!(event.event_type, SearchEventType::CrossProjectSearchExecuted);
        assert!(event.data.is_some());

        let data = event.data.unwrap();
        assert_eq!(data["query"], "test query");
        assert_eq!(data["project_count"], 5);
        assert_eq!(data["result_count"], 42);
    }

    #[test]
    fn test_settings_changed_event() {
        let settings_id = Uuid::new_v4();
        let event = SearchEvent::settings_changed(
            settings_id,
            "max_results",
            serde_json::json!(100),
            serde_json::json!(200),
        );

        assert_eq!(event.event_type, SearchEventType::SearchSettingsChanged);
        let data = event.data.unwrap();
        assert_eq!(data["field"], "max_results");
    }

    #[test]
    fn test_access_denied_event() {
        let user_id = Uuid::new_v4();
        let resource_id = Uuid::new_v4();
        let event = SearchEvent::access_denied(user_id, resource_id, "insufficient permissions");

        assert_eq!(event.event_type, SearchEventType::SearchAccessDenied);
        let data = event.data.unwrap();
        assert_eq!(data["reason"], "insufficient permissions");
    }

    #[test]
    fn test_domain_event_impl() {
        let event = SearchEvent::cross_project_search_executed(Uuid::new_v4(), "query", 1, 10);

        assert_eq!(event.event_type(), "cross_project_search_executed");
        assert!(event.data().is_some());
    }
}
