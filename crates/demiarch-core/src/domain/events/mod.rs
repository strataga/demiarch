//! Domain event infrastructure
//!
//! Provides base traits and types for domain events across all aggregates.
//! Events provide an audit trail and enable loose coupling between components.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Base trait for all domain events
///
/// Domain events represent something that happened in the domain.
/// They are immutable facts about the past.
pub trait DomainEvent: Send + Sync {
    /// Get the event type as a string
    fn event_type(&self) -> &str;

    /// Get the aggregate ID this event belongs to
    fn aggregate_id(&self) -> Uuid;

    /// Get the timestamp when this event occurred
    fn timestamp(&self) -> DateTime<Utc>;

    /// Get optional event data as JSON
    fn data(&self) -> Option<&serde_json::Value>;
}

/// Publisher trait for emitting domain events
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a domain event
    async fn publish(&self, event: &dyn DomainEvent) -> Result<()>;

    /// Publish multiple events in order
    async fn publish_all(&self, events: &[&dyn DomainEvent]) -> Result<()> {
        for event in events {
            self.publish(*event).await?;
        }
        Ok(())
    }
}

/// Subscriber trait for receiving domain events
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Handle a domain event
    async fn handle(&self, event: &dyn DomainEvent) -> Result<()>;

    /// Get the event types this subscriber is interested in
    fn subscribed_events(&self) -> &[&str];
}

/// A simple in-memory event store for recording events
#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    events: std::sync::RwLock<Vec<StoredEvent>>,
}

/// A stored event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Unique event ID
    pub id: Uuid,
    /// Aggregate ID this event belongs to
    pub aggregate_id: Uuid,
    /// Event type string
    pub event_type: String,
    /// Event data as JSON
    pub data: Option<serde_json::Value>,
    /// When the event was created
    pub created_at: DateTime<Utc>,
}

impl StoredEvent {
    /// Create a new stored event
    pub fn new(
        aggregate_id: Uuid,
        event_type: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            aggregate_id,
            event_type: event_type.into(),
            data,
            created_at: Utc::now(),
        }
    }

    /// Create from a domain event
    pub fn from_event(event: &dyn DomainEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            aggregate_id: event.aggregate_id(),
            event_type: event.event_type().to_string(),
            data: event.data().cloned(),
            created_at: event.timestamp(),
        }
    }
}

impl DomainEvent for StoredEvent {
    fn event_type(&self) -> &str {
        &self.event_type
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

impl InMemoryEventStore {
    /// Create a new in-memory event store
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an event
    pub fn store(&self, event: StoredEvent) {
        self.events.write().unwrap().push(event);
    }

    /// Get events for an aggregate
    pub fn events_for(&self, aggregate_id: Uuid) -> Vec<StoredEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .cloned()
            .collect()
    }

    /// Get all events
    pub fn all_events(&self) -> Vec<StoredEvent> {
        self.events.read().unwrap().clone()
    }

    /// Get events by type
    pub fn events_by_type(&self, event_type: &str) -> Vec<StoredEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Clear all events
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventStore {
    async fn publish(&self, event: &dyn DomainEvent) -> Result<()> {
        self.store(StoredEvent::from_event(event));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEvent {
        id: Uuid,
        event_type: String,
        data: Option<serde_json::Value>,
        timestamp: DateTime<Utc>,
    }

    impl DomainEvent for TestEvent {
        fn event_type(&self) -> &str {
            &self.event_type
        }

        fn aggregate_id(&self) -> Uuid {
            self.id
        }

        fn timestamp(&self) -> DateTime<Utc> {
            self.timestamp
        }

        fn data(&self) -> Option<&serde_json::Value> {
            self.data.as_ref()
        }
    }

    #[test]
    fn test_stored_event_creation() {
        let aggregate_id = Uuid::new_v4();
        let event = StoredEvent::new(aggregate_id, "test_event", Some(serde_json::json!({"key": "value"})));

        assert_eq!(event.aggregate_id, aggregate_id);
        assert_eq!(event.event_type, "test_event");
        assert!(event.data.is_some());
    }

    #[test]
    fn test_in_memory_event_store() {
        let store = InMemoryEventStore::new();
        let aggregate_id = Uuid::new_v4();

        store.store(StoredEvent::new(aggregate_id, "event1", None));
        store.store(StoredEvent::new(aggregate_id, "event2", None));
        store.store(StoredEvent::new(Uuid::new_v4(), "event3", None));

        let events = store.events_for(aggregate_id);
        assert_eq!(events.len(), 2);

        let all = store.all_events();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_events_by_type() {
        let store = InMemoryEventStore::new();

        store.store(StoredEvent::new(Uuid::new_v4(), "type_a", None));
        store.store(StoredEvent::new(Uuid::new_v4(), "type_b", None));
        store.store(StoredEvent::new(Uuid::new_v4(), "type_a", None));

        let type_a_events = store.events_by_type("type_a");
        assert_eq!(type_a_events.len(), 2);
    }

    #[tokio::test]
    async fn test_event_publisher() {
        let store = InMemoryEventStore::new();
        let event = TestEvent {
            id: Uuid::new_v4(),
            event_type: "test".to_string(),
            data: None,
            timestamp: Utc::now(),
        };

        store.publish(&event).await.unwrap();

        let stored = store.all_events();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].event_type, "test");
    }
}
