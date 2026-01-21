//! Repository trait for knowledge graph persistence
//!
//! This module defines the trait for knowledge graph storage operations.
//! The trait abstracts over different storage backends (SQLite, etc.).

use async_trait::async_trait;

use crate::error::Result;

use super::entity::{EntityType, KnowledgeEntity};
use super::relationship::{KnowledgeRelationship, RelationshipType};

/// Repository trait for knowledge graph persistence
///
/// Provides CRUD operations for entities and relationships,
/// plus graph traversal and search operations.
#[async_trait]
pub trait KnowledgeGraphRepository: Send + Sync {
    // ========== Entity Operations ==========

    /// Save a knowledge entity (insert or update)
    async fn save_entity(&self, entity: &KnowledgeEntity) -> Result<()>;

    /// Get an entity by ID
    async fn get_entity(&self, id: &str) -> Result<Option<KnowledgeEntity>>;

    /// Get an entity by canonical name
    async fn get_entity_by_canonical_name(&self, canonical_name: &str) -> Result<Option<KnowledgeEntity>>;

    /// List all entities
    async fn list_entities(&self) -> Result<Vec<KnowledgeEntity>>;

    /// List entities by type
    async fn list_entities_by_type(&self, entity_type: EntityType) -> Result<Vec<KnowledgeEntity>>;

    /// Delete an entity by ID
    async fn delete_entity(&self, id: &str) -> Result<bool>;

    /// Count entities
    async fn count_entities(&self) -> Result<u64>;

    // ========== Relationship Operations ==========

    /// Save a relationship (insert or update)
    async fn save_relationship(&self, relationship: &KnowledgeRelationship) -> Result<()>;

    /// Get a relationship by ID
    async fn get_relationship(&self, id: &str) -> Result<Option<KnowledgeRelationship>>;

    /// Get relationship between two entities with a specific type
    async fn get_relationship_between(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: RelationshipType,
    ) -> Result<Option<KnowledgeRelationship>>;

    /// List all relationships for an entity (as source or target)
    async fn list_relationships_for_entity(&self, entity_id: &str) -> Result<Vec<KnowledgeRelationship>>;

    /// List outgoing relationships from an entity
    async fn list_outgoing_relationships(&self, entity_id: &str) -> Result<Vec<KnowledgeRelationship>>;

    /// List incoming relationships to an entity
    async fn list_incoming_relationships(&self, entity_id: &str) -> Result<Vec<KnowledgeRelationship>>;

    /// Delete a relationship by ID
    async fn delete_relationship(&self, id: &str) -> Result<bool>;

    /// Count relationships
    async fn count_relationships(&self) -> Result<u64>;

    // ========== Graph Traversal Operations ==========

    /// Get entities within N hops from a starting entity
    ///
    /// Uses recursive CTE for efficient graph traversal.
    async fn get_neighborhood(
        &self,
        start_entity_id: &str,
        max_depth: u32,
        relationship_types: Option<&[RelationshipType]>,
    ) -> Result<Vec<EntityWithDistance>>;

    /// Find the shortest path between two entities
    async fn find_path(
        &self,
        source_id: &str,
        target_id: &str,
        max_depth: u32,
    ) -> Result<Option<Vec<PathStep>>>;

    /// Get entities connected by specific relationship type
    async fn get_connected_entities(
        &self,
        entity_id: &str,
        relationship_type: RelationshipType,
        direction: TraversalDirection,
    ) -> Result<Vec<KnowledgeEntity>>;

    // ========== Search Operations ==========

    /// Full-text search on entities
    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeEntity>>;

    /// Get entities linked to a skill
    async fn get_entities_for_skill(&self, skill_id: &str) -> Result<Vec<KnowledgeEntity>>;

    /// Get skills linked to an entity
    async fn get_skills_for_entity(&self, entity_id: &str) -> Result<Vec<String>>;

    // ========== Skill-Entity Link Operations ==========

    /// Link a skill to an entity
    async fn link_skill_to_entity(
        &self,
        skill_id: &str,
        entity_id: &str,
        relevance: f32,
    ) -> Result<()>;

    /// Unlink a skill from an entity
    async fn unlink_skill_from_entity(&self, skill_id: &str, entity_id: &str) -> Result<bool>;

    // ========== Entity Embedding Operations ==========

    /// Save an embedding for an entity
    async fn save_entity_embedding(
        &self,
        entity_id: &str,
        embedding: &[f32],
        model: &str,
    ) -> Result<()>;

    /// Get the embedding for an entity
    async fn get_entity_embedding(
        &self,
        entity_id: &str,
        model: &str,
    ) -> Result<Option<Vec<f32>>>;

    /// Semantic search on entities using embeddings
    async fn semantic_search_entities(
        &self,
        query_embedding: &[f32],
        model: &str,
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<EntitySearchResult>>;

    // ========== Statistics ==========

    /// Get graph statistics
    async fn get_stats(&self) -> Result<KnowledgeGraphStats>;
}

/// Direction for graph traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDirection {
    /// Follow outgoing edges (source -> target)
    Outgoing,
    /// Follow incoming edges (target -> source)
    Incoming,
    /// Follow edges in both directions
    Both,
}

/// Entity with distance from a starting point in graph traversal
#[derive(Debug, Clone)]
pub struct EntityWithDistance {
    /// The entity
    pub entity: KnowledgeEntity,
    /// Distance (number of hops) from the starting entity
    pub distance: u32,
    /// Path taken to reach this entity (entity IDs)
    pub path: Vec<String>,
}

/// Step in a path between entities
#[derive(Debug, Clone)]
pub struct PathStep {
    /// Entity at this step
    pub entity_id: String,
    /// Relationship used to reach this entity (None for starting entity)
    pub relationship: Option<PathRelationship>,
}

/// Relationship info in a path
#[derive(Debug, Clone)]
pub struct PathRelationship {
    /// Relationship ID
    pub relationship_id: String,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Weight of the relationship
    pub weight: f32,
}

/// Result from semantic search on entities
#[derive(Debug, Clone)]
pub struct EntitySearchResult {
    /// The matched entity
    pub entity: KnowledgeEntity,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
}

/// Statistics about the knowledge graph
#[derive(Debug, Clone, Default)]
pub struct KnowledgeGraphStats {
    /// Total number of entities
    pub total_entities: u64,
    /// Total number of relationships
    pub total_relationships: u64,
    /// Total number of skill-entity links
    pub total_skill_links: u64,
    /// Entities by type
    pub entities_by_type: Vec<(EntityType, u64)>,
    /// Relationships by type
    pub relationships_by_type: Vec<(RelationshipType, u64)>,
    /// Average confidence across entities
    pub average_entity_confidence: f32,
    /// Average weight across relationships
    pub average_relationship_weight: f32,
    /// Number of entities with embeddings
    pub entities_with_embeddings: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traversal_direction() {
        assert_eq!(TraversalDirection::Outgoing, TraversalDirection::Outgoing);
        assert_ne!(TraversalDirection::Incoming, TraversalDirection::Both);
    }

    #[test]
    fn test_entity_with_distance() {
        let entity = KnowledgeEntity::new("test", EntityType::Concept);
        let with_distance = EntityWithDistance {
            entity,
            distance: 2,
            path: vec!["a".into(), "b".into(), "c".into()],
        };

        assert_eq!(with_distance.distance, 2);
        assert_eq!(with_distance.path.len(), 3);
    }
}
