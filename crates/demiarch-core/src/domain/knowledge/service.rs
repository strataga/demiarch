//! Knowledge graph service
//!
//! This module provides the main service for the GraphRAG system,
//! orchestrating entity extraction, graph operations, and search.

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::error::Result;
use crate::llm::LlmClient;
use crate::skills::LearnedSkill;

use super::entity::{EntityType, KnowledgeEntity};
use super::event::KnowledgeEvent;
use super::extractor::EntityExtractor;
use super::relationship::{KnowledgeRelationship, RelationshipEvidence, RelationshipType};
use super::repository::{
    EntitySearchResult, EntityWithDistance, KnowledgeGraphRepository, KnowledgeGraphStats,
    TraversalDirection,
};

/// Knowledge graph service for GraphRAG operations
///
/// Provides high-level operations for:
/// - Cognifying skills (Extract-Connect-Load pipeline)
/// - Graph traversal and exploration
/// - Hybrid search combining FTS, embeddings, and graph signals
pub struct KnowledgeGraphService<R: KnowledgeGraphRepository> {
    /// Repository for persistence
    repository: Arc<R>,
    /// Entity extractor for skill analysis
    extractor: Option<EntityExtractor>,
    /// Event store for audit trail
    events: Vec<KnowledgeEvent>,
}

impl<R: KnowledgeGraphRepository> KnowledgeGraphService<R> {
    /// Create a new knowledge graph service
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
            extractor: None,
            events: Vec::new(),
        }
    }

    /// Configure the LLM client for entity extraction
    pub fn with_llm_client(mut self, llm_client: Arc<LlmClient>) -> Self {
        self.extractor = Some(EntityExtractor::new(llm_client));
        self
    }

    /// Set the entity extractor directly
    pub fn with_extractor(mut self, extractor: EntityExtractor) -> Self {
        self.extractor = Some(extractor);
        self
    }

    // ========== ECL Pipeline (Extract-Connect-Load) ==========

    /// Cognify a skill: extract entities and relationships, then add to graph
    ///
    /// This implements the cognee-inspired ECL pipeline:
    /// 1. Extract entities from skill content
    /// 2. Connect entities with relationships
    /// 3. Load into the knowledge graph with deduplication
    pub async fn cognify_skill(&mut self, skill: &LearnedSkill) -> Result<CognifyResult> {
        let extractor = self
            .extractor
            .as_ref()
            .ok_or_else(|| crate::error::Error::Other("LLM client not configured".into()))?;

        info!(skill_id = %skill.id, skill_name = %skill.name, "Cognifying skill");

        // Step 1: Extract entities and relationships
        let extraction = extractor.extract_from_skill(skill).await?;

        // Step 2: Deduplicate and merge entities
        let (entities, entity_mapping) = self.merge_entities(extraction.entities).await?;

        // Step 3: Remap relationships to use canonical entity IDs
        let relationships = self
            .remap_relationships(extraction.relationships, &entity_mapping)
            .await?;

        // Step 4: Save to graph
        for entity in &entities {
            self.repository.save_entity(entity).await?;
        }

        for relationship in &relationships {
            self.repository.save_relationship(relationship).await?;
        }

        // Step 5: Link skill to entities
        for entity in &entities {
            self.repository
                .link_skill_to_entity(&skill.id, &entity.id, 0.8)
                .await?;
        }

        // Record event
        self.events.push(KnowledgeEvent::skill_cognified(
            &skill.id,
            entities.iter().map(|e| e.id.clone()).collect(),
            relationships.iter().map(|r| r.id.clone()).collect(),
        ));

        let result = CognifyResult {
            skill_id: skill.id.clone(),
            entities_created: entities
                .iter()
                .filter(|e| e.source_skill_ids.len() == 1)
                .count(),
            entities_merged: entities
                .iter()
                .filter(|e| e.source_skill_ids.len() > 1)
                .count(),
            relationships_created: relationships.len(),
        };

        info!(
            skill_id = %skill.id,
            entities_created = result.entities_created,
            entities_merged = result.entities_merged,
            relationships = result.relationships_created,
            "Skill cognified"
        );

        Ok(result)
    }

    /// Cognify multiple skills (batch operation)
    pub async fn cognify_skills(&mut self, skills: &[LearnedSkill]) -> Result<Vec<CognifyResult>> {
        let mut results = Vec::with_capacity(skills.len());

        for skill in skills {
            match self.cognify_skill(skill).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!(skill_id = %skill.id, error = %e, "Failed to cognify skill");
                }
            }
        }

        Ok(results)
    }

    /// Merge new entities with existing ones based on canonical name
    async fn merge_entities(
        &self,
        entities: Vec<KnowledgeEntity>,
    ) -> Result<(
        Vec<KnowledgeEntity>,
        std::collections::HashMap<String, String>,
    )> {
        let mut merged_entities = Vec::new();
        let mut id_mapping = std::collections::HashMap::new();

        for entity in entities {
            // Check if entity with same canonical name exists
            if let Some(existing) = self
                .repository
                .get_entity_by_canonical_name(&entity.canonical_name)
                .await?
            {
                // Merge: add source skill to existing entity
                let mut updated = existing.clone();
                for skill_id in &entity.source_skill_ids {
                    updated.add_source_skill(skill_id.clone());
                }

                // Merge aliases
                for alias in &entity.aliases {
                    updated.add_alias(alias.clone());
                }

                // Boost confidence (capped at 1.0)
                updated.update_confidence(0.1);

                id_mapping.insert(entity.id.clone(), updated.id.clone());
                merged_entities.push(updated);

                debug!(
                    entity_id = %existing.id,
                    entity_name = %existing.name,
                    "Merged with existing entity"
                );
            } else {
                // New entity
                id_mapping.insert(entity.id.clone(), entity.id.clone());
                merged_entities.push(entity);
            }
        }

        Ok((merged_entities, id_mapping))
    }

    /// Remap relationship entity IDs after merging
    async fn remap_relationships(
        &self,
        relationships: Vec<KnowledgeRelationship>,
        entity_mapping: &std::collections::HashMap<String, String>,
    ) -> Result<Vec<KnowledgeRelationship>> {
        let mut remapped = Vec::new();

        for rel in relationships {
            let source_id = entity_mapping
                .get(&rel.source_entity_id)
                .cloned()
                .unwrap_or(rel.source_entity_id.clone());

            let target_id = entity_mapping
                .get(&rel.target_entity_id)
                .cloned()
                .unwrap_or(rel.target_entity_id.clone());

            // Check if relationship already exists
            if let Some(existing) = self
                .repository
                .get_relationship_between(&source_id, &target_id, rel.relationship_type)
                .await?
            {
                // Update existing relationship
                let mut updated = existing;
                for evidence in rel.evidence {
                    updated.add_evidence(evidence);
                }
                remapped.push(updated);
            } else {
                // Create new relationship with remapped IDs
                let new_rel =
                    KnowledgeRelationship::new(&source_id, &target_id, rel.relationship_type)
                        .with_weight(rel.weight)
                        .with_evidence(rel.evidence);
                remapped.push(new_rel);
            }
        }

        Ok(remapped)
    }

    // ========== Graph Exploration ==========

    /// Get the neighborhood of an entity (entities within N hops)
    pub async fn get_neighborhood(
        &self,
        entity_id: &str,
        max_depth: u32,
    ) -> Result<Vec<EntityWithDistance>> {
        self.repository
            .get_neighborhood(entity_id, max_depth, None)
            .await
    }

    /// Get the neighborhood filtered by relationship types
    pub async fn get_neighborhood_filtered(
        &self,
        entity_id: &str,
        max_depth: u32,
        relationship_types: &[RelationshipType],
    ) -> Result<Vec<EntityWithDistance>> {
        self.repository
            .get_neighborhood(entity_id, max_depth, Some(relationship_types))
            .await
    }

    /// Get entities directly connected to an entity
    pub async fn get_connected(
        &self,
        entity_id: &str,
        relationship_type: RelationshipType,
        direction: TraversalDirection,
    ) -> Result<Vec<KnowledgeEntity>> {
        self.repository
            .get_connected_entities(entity_id, relationship_type, direction)
            .await
    }

    /// Find path between two entities
    pub async fn find_path(
        &self,
        source_id: &str,
        target_id: &str,
        max_depth: u32,
    ) -> Result<Option<Vec<super::repository::PathStep>>> {
        self.repository
            .find_path(source_id, target_id, max_depth)
            .await
    }

    // ========== Entity Operations ==========

    /// Create or update an entity
    pub async fn save_entity(&self, entity: &KnowledgeEntity) -> Result<()> {
        self.repository.save_entity(entity).await
    }

    /// Get an entity by ID
    pub async fn get_entity(&self, id: &str) -> Result<Option<KnowledgeEntity>> {
        self.repository.get_entity(id).await
    }

    /// List all entities
    pub async fn list_entities(&self) -> Result<Vec<KnowledgeEntity>> {
        self.repository.list_entities().await
    }

    /// List entities by type
    pub async fn list_entities_by_type(
        &self,
        entity_type: EntityType,
    ) -> Result<Vec<KnowledgeEntity>> {
        self.repository.list_entities_by_type(entity_type).await
    }

    /// Delete an entity
    pub async fn delete_entity(&self, id: &str) -> Result<bool> {
        self.repository.delete_entity(id).await
    }

    // ========== Relationship Operations ==========

    /// Create or update a relationship
    pub async fn save_relationship(&self, relationship: &KnowledgeRelationship) -> Result<()> {
        self.repository.save_relationship(relationship).await
    }

    /// Get a relationship by ID
    pub async fn get_relationship(&self, id: &str) -> Result<Option<KnowledgeRelationship>> {
        self.repository.get_relationship(id).await
    }

    /// List relationships for an entity
    pub async fn list_relationships_for_entity(
        &self,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeRelationship>> {
        self.repository
            .list_relationships_for_entity(entity_id)
            .await
    }

    /// Infer a relationship between two entities
    pub async fn infer_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: RelationshipType,
        evidence: RelationshipEvidence,
    ) -> Result<KnowledgeRelationship> {
        // Check if relationship exists
        if let Some(mut existing) = self
            .repository
            .get_relationship_between(source_id, target_id, relationship_type)
            .await?
        {
            existing.add_evidence(evidence);
            self.repository.save_relationship(&existing).await?;
            Ok(existing)
        } else {
            let rel = KnowledgeRelationship::new(source_id, target_id, relationship_type)
                .with_evidence(vec![evidence]);
            self.repository.save_relationship(&rel).await?;
            Ok(rel)
        }
    }

    // ========== Search Operations ==========

    /// Search entities by text
    pub async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeEntity>> {
        self.repository.search_entities(query, limit).await
    }

    /// Semantic search on entities
    pub async fn semantic_search(
        &self,
        query_embedding: &[f32],
        model: &str,
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<EntitySearchResult>> {
        self.repository
            .semantic_search_entities(query_embedding, model, limit, min_similarity)
            .await
    }

    /// Get entities for a skill
    pub async fn get_entities_for_skill(&self, skill_id: &str) -> Result<Vec<KnowledgeEntity>> {
        self.repository.get_entities_for_skill(skill_id).await
    }

    /// Get skills for an entity
    pub async fn get_skills_for_entity(&self, entity_id: &str) -> Result<Vec<String>> {
        self.repository.get_skills_for_entity(entity_id).await
    }

    // ========== Embedding Operations ==========

    /// Save an embedding for an entity
    pub async fn save_entity_embedding(
        &self,
        entity_id: &str,
        embedding: &[f32],
        model: &str,
    ) -> Result<()> {
        self.repository
            .save_entity_embedding(entity_id, embedding, model)
            .await
    }

    /// Get an embedding for an entity
    pub async fn get_entity_embedding(
        &self,
        entity_id: &str,
        model: &str,
    ) -> Result<Option<Vec<f32>>> {
        self.repository.get_entity_embedding(entity_id, model).await
    }

    // ========== Confidence Propagation ==========

    /// Propagate confidence changes through the graph
    ///
    /// When an entity's confidence changes, propagate a dampened change
    /// to connected entities. This implements the "self-improving graph" pattern.
    pub async fn propagate_confidence(
        &mut self,
        entity_id: &str,
        delta: f32,
        max_hops: u32,
    ) -> Result<Vec<(String, f32)>> {
        let mut affected = Vec::new();
        let dampening_factor: f32 = 0.5; // Reduce delta by half at each hop

        let neighbors = self.get_neighborhood(entity_id, max_hops).await?;

        for neighbor in neighbors {
            // Calculate dampened delta based on distance
            let hop_dampening = dampening_factor.powi(neighbor.distance as i32);
            let adjusted_delta = delta * hop_dampening;

            if adjusted_delta.abs() > 0.01 {
                // Only apply significant changes
                let mut entity = neighbor.entity;
                entity.update_confidence(adjusted_delta);
                self.repository.save_entity(&entity).await?;
                affected.push((entity.id.clone(), adjusted_delta));
            }
        }

        if !affected.is_empty() {
            self.events.push(KnowledgeEvent::ConfidencePropagated {
                trigger_entity_id: entity_id.to_string(),
                affected_entities: affected.clone(),
                reason: format!("Propagated delta {} with {} hops", delta, max_hops),
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(affected)
    }

    // ========== Statistics ==========

    /// Get graph statistics
    pub async fn get_stats(&self) -> Result<KnowledgeGraphStats> {
        self.repository.get_stats().await
    }

    /// Get collected events
    pub fn get_events(&self) -> &[KnowledgeEvent] {
        &self.events
    }

    /// Clear collected events
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

/// Result of cognifying a skill
#[derive(Debug, Clone)]
pub struct CognifyResult {
    /// ID of the cognified skill
    pub skill_id: String,
    /// Number of new entities created
    pub entities_created: usize,
    /// Number of entities merged with existing
    pub entities_merged: usize,
    /// Number of relationships created
    pub relationships_created: usize,
}

impl CognifyResult {
    /// Total entities processed
    pub fn total_entities(&self) -> usize {
        self.entities_created + self.entities_merged
    }

    /// Check if any entities were processed
    pub fn has_entities(&self) -> bool {
        self.total_entities() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::knowledge::SqliteKnowledgeGraphRepository;
    use crate::storage::migrations::run_migrations;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_service() -> KnowledgeGraphService<SqliteKnowledgeGraphRepository> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let repo = Arc::new(SqliteKnowledgeGraphRepository::new(pool));
        KnowledgeGraphService::new(repo)
    }

    #[tokio::test]
    async fn test_save_and_get_entity() {
        let service = setup_test_service().await;

        let entity = KnowledgeEntity::new("tokio", EntityType::Library)
            .with_description("Async runtime for Rust");

        service.save_entity(&entity).await.unwrap();

        let retrieved = service.get_entity(&entity.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "tokio");
    }

    #[tokio::test]
    async fn test_infer_relationship() {
        let service = setup_test_service().await;

        let e1 = KnowledgeEntity::new("tokio", EntityType::Library);
        let e2 = KnowledgeEntity::new("async-trait", EntityType::Library);

        service.save_entity(&e1).await.unwrap();
        service.save_entity(&e2).await.unwrap();

        let evidence =
            RelationshipEvidence::from_skill_cooccurrence("skill-1", "Both used together");

        let rel = service
            .infer_relationship(&e1.id, &e2.id, RelationshipType::Uses, evidence)
            .await
            .unwrap();

        assert_eq!(rel.source_entity_id, e1.id);
        assert_eq!(rel.target_entity_id, e2.id);
        assert_eq!(rel.evidence.len(), 1);
    }

    #[tokio::test]
    async fn test_get_neighborhood() {
        let service = setup_test_service().await;

        // Create a chain: A -> B -> C
        let a = KnowledgeEntity::new("A", EntityType::Concept);
        let b = KnowledgeEntity::new("B", EntityType::Concept);
        let c = KnowledgeEntity::new("C", EntityType::Concept);

        service.save_entity(&a).await.unwrap();
        service.save_entity(&b).await.unwrap();
        service.save_entity(&c).await.unwrap();

        let rel_ab = KnowledgeRelationship::new(&a.id, &b.id, RelationshipType::Uses);
        let rel_bc = KnowledgeRelationship::new(&b.id, &c.id, RelationshipType::Uses);

        service.save_relationship(&rel_ab).await.unwrap();
        service.save_relationship(&rel_bc).await.unwrap();

        // 1-hop from A should find B
        let neighbors = service.get_neighborhood(&a.id, 1).await.unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].entity.name, "B");

        // 2-hop from A should find B and C
        let neighbors = service.get_neighborhood(&a.id, 2).await.unwrap();
        assert_eq!(neighbors.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let service = setup_test_service().await;

        let entity = KnowledgeEntity::new("test", EntityType::Concept);
        service.save_entity(&entity).await.unwrap();

        let stats = service.get_stats().await.unwrap();
        assert_eq!(stats.total_entities, 1);
        assert_eq!(stats.total_relationships, 0);
    }

    #[tokio::test]
    async fn test_cognify_result() {
        let result = CognifyResult {
            skill_id: "skill-1".into(),
            entities_created: 3,
            entities_merged: 2,
            relationships_created: 4,
        };

        assert_eq!(result.total_entities(), 5);
        assert!(result.has_entities());
    }
}
