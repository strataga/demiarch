//! Domain events for the knowledge graph
//!
//! This module defines events that occur in the knowledge graph system.
//! Events are used for audit trails and loose coupling between components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::entity::EntityType;
use super::relationship::RelationshipType;

/// Events that can occur in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum KnowledgeEvent {
    /// A new entity was created
    EntityCreated {
        entity_id: String,
        entity_type: EntityType,
        name: String,
        source_skill_id: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// An entity was updated
    EntityUpdated {
        entity_id: String,
        changes: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// An entity was merged with another
    EntityMerged {
        source_entity_id: String,
        target_entity_id: String,
        timestamp: DateTime<Utc>,
    },
    /// An entity was deleted
    EntityDeleted {
        entity_id: String,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// A new relationship was created
    RelationshipCreated {
        relationship_id: String,
        source_entity_id: String,
        target_entity_id: String,
        relationship_type: RelationshipType,
        timestamp: DateTime<Utc>,
    },
    /// A relationship was strengthened (weight increased)
    RelationshipStrengthened {
        relationship_id: String,
        old_weight: f32,
        new_weight: f32,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// A relationship was weakened (weight decreased)
    RelationshipWeakened {
        relationship_id: String,
        old_weight: f32,
        new_weight: f32,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// A relationship was deleted
    RelationshipDeleted {
        relationship_id: String,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Entity extraction was performed on a skill
    SkillCognified {
        skill_id: String,
        entities_extracted: Vec<String>,
        relationships_inferred: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// Confidence propagation was applied
    ConfidencePropagated {
        trigger_entity_id: String,
        affected_entities: Vec<(String, f32)>, // (entity_id, delta)
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Graph search was performed
    GraphSearched {
        query: String,
        result_count: usize,
        hop_depth: u32,
        timestamp: DateTime<Utc>,
    },
}

impl KnowledgeEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::EntityCreated { timestamp, .. }
            | Self::EntityUpdated { timestamp, .. }
            | Self::EntityMerged { timestamp, .. }
            | Self::EntityDeleted { timestamp, .. }
            | Self::RelationshipCreated { timestamp, .. }
            | Self::RelationshipStrengthened { timestamp, .. }
            | Self::RelationshipWeakened { timestamp, .. }
            | Self::RelationshipDeleted { timestamp, .. }
            | Self::SkillCognified { timestamp, .. }
            | Self::ConfidencePropagated { timestamp, .. }
            | Self::GraphSearched { timestamp, .. } => *timestamp,
        }
    }

    /// Get the event type name
    pub fn event_type_name(&self) -> &'static str {
        match self {
            Self::EntityCreated { .. } => "entity_created",
            Self::EntityUpdated { .. } => "entity_updated",
            Self::EntityMerged { .. } => "entity_merged",
            Self::EntityDeleted { .. } => "entity_deleted",
            Self::RelationshipCreated { .. } => "relationship_created",
            Self::RelationshipStrengthened { .. } => "relationship_strengthened",
            Self::RelationshipWeakened { .. } => "relationship_weakened",
            Self::RelationshipDeleted { .. } => "relationship_deleted",
            Self::SkillCognified { .. } => "skill_cognified",
            Self::ConfidencePropagated { .. } => "confidence_propagated",
            Self::GraphSearched { .. } => "graph_searched",
        }
    }

    /// Get the primary entity/aggregate ID for this event
    pub fn aggregate_id(&self) -> Option<&str> {
        match self {
            Self::EntityCreated { entity_id, .. }
            | Self::EntityUpdated { entity_id, .. }
            | Self::EntityDeleted { entity_id, .. } => Some(entity_id),
            Self::EntityMerged {
                target_entity_id, ..
            } => Some(target_entity_id),
            Self::RelationshipCreated {
                relationship_id, ..
            }
            | Self::RelationshipStrengthened {
                relationship_id, ..
            }
            | Self::RelationshipWeakened {
                relationship_id, ..
            }
            | Self::RelationshipDeleted {
                relationship_id, ..
            } => Some(relationship_id),
            Self::SkillCognified { skill_id, .. } => Some(skill_id),
            Self::ConfidencePropagated {
                trigger_entity_id, ..
            } => Some(trigger_entity_id),
            Self::GraphSearched { .. } => None,
        }
    }

    /// Create a new EntityCreated event
    pub fn entity_created(
        entity_id: impl Into<String>,
        entity_type: EntityType,
        name: impl Into<String>,
        source_skill_id: Option<String>,
    ) -> Self {
        Self::EntityCreated {
            entity_id: entity_id.into(),
            entity_type,
            name: name.into(),
            source_skill_id,
            timestamp: Utc::now(),
        }
    }

    /// Create a new RelationshipCreated event
    pub fn relationship_created(
        relationship_id: impl Into<String>,
        source_entity_id: impl Into<String>,
        target_entity_id: impl Into<String>,
        relationship_type: RelationshipType,
    ) -> Self {
        Self::RelationshipCreated {
            relationship_id: relationship_id.into(),
            source_entity_id: source_entity_id.into(),
            target_entity_id: target_entity_id.into(),
            relationship_type,
            timestamp: Utc::now(),
        }
    }

    /// Create a new SkillCognified event
    pub fn skill_cognified(
        skill_id: impl Into<String>,
        entities_extracted: Vec<String>,
        relationships_inferred: Vec<String>,
    ) -> Self {
        Self::SkillCognified {
            skill_id: skill_id.into(),
            entities_extracted,
            relationships_inferred,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = KnowledgeEvent::entity_created(
            "entity-123",
            EntityType::Library,
            "tokio",
            Some("skill-456".into()),
        );

        assert_eq!(event.event_type_name(), "entity_created");
        assert_eq!(event.aggregate_id(), Some("entity-123"));
    }

    #[test]
    fn test_relationship_event() {
        let event = KnowledgeEvent::relationship_created(
            "rel-123",
            "entity-1",
            "entity-2",
            RelationshipType::Uses,
        );

        assert_eq!(event.event_type_name(), "relationship_created");
        assert_eq!(event.aggregate_id(), Some("rel-123"));
    }

    #[test]
    fn test_skill_cognified_event() {
        let event = KnowledgeEvent::skill_cognified(
            "skill-123",
            vec!["entity-1".into(), "entity-2".into()],
            vec!["rel-1".into()],
        );

        assert_eq!(event.event_type_name(), "skill_cognified");
        assert_eq!(event.aggregate_id(), Some("skill-123"));
    }
}
