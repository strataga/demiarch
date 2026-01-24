//! Context enricher for graph-aware prompting
//!
//! This module provides functionality to enrich prompts with relevant
//! knowledge graph context, enabling more informed code generation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::{debug, info};

use crate::error::Result;

use super::entity::{EntityType, KnowledgeEntity};
use super::relationship::{KnowledgeRelationship, RelationshipType};
use super::repository::{EntityWithDistance, KnowledgeGraphRepository};

/// Configuration for context enrichment
#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    /// Maximum number of entities to include in context
    pub max_entities: usize,
    /// Maximum graph traversal depth
    pub max_depth: u32,
    /// Minimum entity confidence to include
    pub min_confidence: f32,
    /// Entity types to prioritize (in order)
    pub priority_types: Vec<EntityType>,
    /// Relationship types to follow
    pub relationship_types: Option<Vec<RelationshipType>>,
    /// Whether to include relationship descriptions
    pub include_relationships: bool,
    /// Maximum tokens for context (approximate)
    pub max_context_tokens: usize,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            max_entities: 10,
            max_depth: 2,
            min_confidence: 0.4,
            priority_types: vec![
                EntityType::Library,
                EntityType::Framework,
                EntityType::Pattern,
                EntityType::Concept,
                EntityType::Technique,
            ],
            relationship_types: None,
            include_relationships: true,
            max_context_tokens: 500,
        }
    }
}

impl EnrichmentConfig {
    /// Create a minimal config for quick enrichment
    pub fn minimal() -> Self {
        Self {
            max_entities: 5,
            max_depth: 1,
            min_confidence: 0.5,
            include_relationships: false,
            max_context_tokens: 200,
            ..Default::default()
        }
    }

    /// Create a rich config for comprehensive context
    pub fn comprehensive() -> Self {
        Self {
            max_entities: 15,
            max_depth: 3,
            min_confidence: 0.3,
            include_relationships: true,
            max_context_tokens: 800,
            ..Default::default()
        }
    }
}

/// Context enrichment result
#[derive(Debug, Clone)]
pub struct EnrichedContext {
    /// Relevant entities found
    pub entities: Vec<EntityContext>,
    /// Relationships between entities
    pub relationships: Vec<RelationshipContext>,
    /// Formatted context string for prompt injection
    pub formatted_context: String,
    /// Query terms that matched
    pub matched_terms: Vec<String>,
    /// Statistics about the enrichment
    pub stats: EnrichmentStats,
}

/// Context information for a single entity
#[derive(Debug, Clone)]
pub struct EntityContext {
    /// The entity
    pub entity: KnowledgeEntity,
    /// Distance from query (0 = direct match)
    pub distance: u32,
    /// Why this entity is relevant
    pub relevance_reason: String,
    /// Related skill IDs
    pub related_skills: Vec<String>,
}

/// Context information for a relationship
#[derive(Debug, Clone)]
pub struct RelationshipContext {
    /// Source entity name
    pub source_name: String,
    /// Target entity name
    pub target_name: String,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Human-readable description
    pub description: String,
}

/// Statistics about the enrichment process
#[derive(Debug, Clone, Default)]
pub struct EnrichmentStats {
    /// Total entities searched
    pub entities_searched: usize,
    /// Entities that matched criteria
    pub entities_matched: usize,
    /// Graph hops traversed
    pub max_depth_reached: u32,
    /// Estimated token count
    pub estimated_tokens: usize,
}

/// Context enricher that traverses the knowledge graph
///
/// This component analyzes queries/tasks to find relevant concepts
/// in the knowledge graph and formats them for prompt injection.
pub struct ContextEnricher<R: KnowledgeGraphRepository> {
    /// Repository for graph access
    repository: Arc<R>,
    /// Configuration
    config: EnrichmentConfig,
}

impl<R: KnowledgeGraphRepository> ContextEnricher<R> {
    /// Create a new context enricher
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
            config: EnrichmentConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(repository: Arc<R>, config: EnrichmentConfig) -> Self {
        Self { repository, config }
    }

    /// Enrich context based on a query string
    ///
    /// Analyzes the query to extract key terms, searches the knowledge graph,
    /// and returns relevant context for prompt injection.
    pub async fn enrich_from_query(&self, query: &str) -> Result<EnrichedContext> {
        info!(query_len = query.len(), "Enriching context from query");

        // Step 1: Extract key terms from query
        let terms = self.extract_terms(query);
        debug!(terms = ?terms, "Extracted terms from query");

        // Step 2: Search for matching entities
        let mut all_entities: HashMap<String, EntityWithDistance> = HashMap::new();
        let mut matched_terms = Vec::new();

        for term in &terms {
            let search_results = self.repository.search_entities(term, 10).await?;

            for entity in search_results {
                if entity.confidence >= self.config.min_confidence {
                    matched_terms.push(term.clone());
                    all_entities
                        .entry(entity.id.clone())
                        .or_insert(EntityWithDistance {
                            entity,
                            distance: 0,
                            path: Vec::new(),
                        });
                }
            }
        }

        // Step 3: Expand via graph traversal
        let direct_entity_ids: Vec<String> = all_entities.keys().cloned().collect();
        for entity_id in direct_entity_ids {
            if all_entities.len() >= self.config.max_entities * 2 {
                break; // Don't gather too many candidates
            }

            let neighbors = self
                .repository
                .get_neighborhood(
                    &entity_id,
                    self.config.max_depth,
                    self.config.relationship_types.as_deref(),
                )
                .await?;

            for neighbor in neighbors {
                if neighbor.entity.confidence >= self.config.min_confidence {
                    all_entities
                        .entry(neighbor.entity.id.clone())
                        .or_insert(neighbor);
                }
            }
        }

        // Step 4: Rank and select top entities
        let selected_entities = self.rank_and_select_entities(all_entities);

        // Step 5: Gather relationships between selected entities
        let relationships = if self.config.include_relationships {
            self.gather_relationships(&selected_entities).await?
        } else {
            Vec::new()
        };

        // Step 6: Format context
        let formatted_context = self.format_context(&selected_entities, &relationships);

        let stats = EnrichmentStats {
            entities_searched: terms.len() * 10, // Approximate
            entities_matched: selected_entities.len(),
            max_depth_reached: self.config.max_depth,
            estimated_tokens: estimate_tokens(&formatted_context),
        };

        Ok(EnrichedContext {
            entities: selected_entities,
            relationships,
            formatted_context,
            matched_terms: matched_terms
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),
            stats,
        })
    }

    /// Enrich context based on specific entity IDs
    ///
    /// Use this when you already know which entities are relevant
    /// (e.g., from a skill lookup).
    pub async fn enrich_from_entities(&self, entity_ids: &[String]) -> Result<EnrichedContext> {
        info!(
            entity_count = entity_ids.len(),
            "Enriching context from entities"
        );

        let mut all_entities: HashMap<String, EntityWithDistance> = HashMap::new();

        // Get the seed entities
        for entity_id in entity_ids {
            if let Some(entity) = self.repository.get_entity(entity_id).await? {
                all_entities.insert(
                    entity.id.clone(),
                    EntityWithDistance {
                        entity,
                        distance: 0,
                        path: Vec::new(),
                    },
                );
            }
        }

        // Expand via traversal
        for entity_id in entity_ids {
            let neighbors = self
                .repository
                .get_neighborhood(
                    entity_id,
                    self.config.max_depth,
                    self.config.relationship_types.as_deref(),
                )
                .await?;

            for neighbor in neighbors {
                if neighbor.entity.confidence >= self.config.min_confidence {
                    all_entities
                        .entry(neighbor.entity.id.clone())
                        .or_insert(neighbor);
                }
            }
        }

        let selected_entities = self.rank_and_select_entities(all_entities);
        let relationships = if self.config.include_relationships {
            self.gather_relationships(&selected_entities).await?
        } else {
            Vec::new()
        };

        let formatted_context = self.format_context(&selected_entities, &relationships);

        let stats = EnrichmentStats {
            entities_searched: entity_ids.len(),
            entities_matched: selected_entities.len(),
            max_depth_reached: self.config.max_depth,
            estimated_tokens: estimate_tokens(&formatted_context),
        };

        Ok(EnrichedContext {
            entities: selected_entities,
            relationships,
            formatted_context,
            matched_terms: Vec::new(),
            stats,
        })
    }

    /// Enrich context for a specific skill
    ///
    /// Gets entities linked to the skill and expands from there.
    pub async fn enrich_for_skill(&self, skill_id: &str) -> Result<EnrichedContext> {
        info!(skill_id = %skill_id, "Enriching context for skill");

        let entities = self.repository.get_entities_for_skill(skill_id).await?;
        let entity_ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();

        self.enrich_from_entities(&entity_ids).await
    }

    /// Extract searchable terms from a query
    fn extract_terms(&self, query: &str) -> Vec<String> {
        // Simple term extraction: split on whitespace and punctuation,
        // filter short words and common stop words
        let stop_words: HashSet<&str> = [
            "the",
            "a",
            "an",
            "is",
            "are",
            "was",
            "were",
            "be",
            "been",
            "being",
            "have",
            "has",
            "had",
            "do",
            "does",
            "did",
            "will",
            "would",
            "could",
            "should",
            "may",
            "might",
            "must",
            "shall",
            "can",
            "need",
            "dare",
            "to",
            "of",
            "in",
            "for",
            "on",
            "with",
            "at",
            "by",
            "from",
            "up",
            "about",
            "into",
            "through",
            "during",
            "before",
            "after",
            "above",
            "below",
            "between",
            "under",
            "again",
            "further",
            "then",
            "once",
            "here",
            "there",
            "when",
            "where",
            "why",
            "how",
            "all",
            "each",
            "few",
            "more",
            "most",
            "other",
            "some",
            "such",
            "no",
            "nor",
            "not",
            "only",
            "own",
            "same",
            "so",
            "than",
            "too",
            "very",
            "just",
            "now",
            "and",
            "but",
            "if",
            "or",
            "because",
            "as",
            "until",
            "while",
            "this",
            "that",
            "these",
            "those",
            "what",
            "which",
            "who",
            "whom",
            "i",
            "me",
            "my",
            "we",
            "our",
            "you",
            "your",
            "it",
            "its",
            "they",
            "them",
            "code",
            "write",
            "create",
            "make",
            "implement",
            "add",
            "use",
            "using",
        ]
        .into_iter()
        .collect();

        query
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|word| word.len() >= 3)
            .filter(|word| !stop_words.contains(word))
            .map(|s| s.to_string())
            .collect()
    }

    /// Rank entities and select the top ones
    fn rank_and_select_entities(
        &self,
        entities: HashMap<String, EntityWithDistance>,
    ) -> Vec<EntityContext> {
        let mut scored: Vec<(EntityWithDistance, f32)> = entities
            .into_values()
            .map(|e| {
                let score = self.score_entity(&e);
                (e, score)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N and convert to EntityContext
        scored
            .into_iter()
            .take(self.config.max_entities)
            .map(|(ewd, _score)| {
                let relevance_reason = self.determine_relevance(&ewd);
                EntityContext {
                    related_skills: ewd.entity.source_skill_ids.clone(),
                    entity: ewd.entity,
                    distance: ewd.distance,
                    relevance_reason,
                }
            })
            .collect()
    }

    /// Score an entity for ranking
    fn score_entity(&self, entity_with_distance: &EntityWithDistance) -> f32 {
        let entity = &entity_with_distance.entity;
        let distance = entity_with_distance.distance;

        // Base score from confidence
        let mut score = entity.confidence;

        // Distance penalty (closer = higher score)
        let distance_penalty = 0.85_f32.powi(distance as i32);
        score *= distance_penalty;

        // Type priority bonus
        if let Some(pos) = self
            .config
            .priority_types
            .iter()
            .position(|t| *t == entity.entity_type)
        {
            // Earlier in priority list = higher bonus
            let type_bonus = 1.0 + (0.1 * (self.config.priority_types.len() - pos) as f32);
            score *= type_bonus;
        }

        // Bonus for entities with descriptions
        if entity.description.is_some() {
            score *= 1.1;
        }

        // Bonus for entities with multiple source skills (more "verified")
        let skill_bonus = 1.0 + (entity.source_skill_ids.len() as f32 * 0.05).min(0.3);
        score *= skill_bonus;

        score
    }

    /// Determine why an entity is relevant
    fn determine_relevance(&self, entity_with_distance: &EntityWithDistance) -> String {
        let entity = &entity_with_distance.entity;
        let distance = entity_with_distance.distance;

        if distance == 0 {
            format!("Direct match for {}", entity.entity_type.as_str())
        } else {
            format!(
                "Related {} ({} hop{})",
                entity.entity_type.as_str(),
                distance,
                if distance > 1 { "s" } else { "" }
            )
        }
    }

    /// Gather relationships between selected entities
    async fn gather_relationships(
        &self,
        entities: &[EntityContext],
    ) -> Result<Vec<RelationshipContext>> {
        let entity_ids: HashSet<&str> = entities.iter().map(|e| e.entity.id.as_str()).collect();
        let mut relationships = Vec::new();

        for entity_ctx in entities {
            let entity_rels = self
                .repository
                .list_relationships_for_entity(&entity_ctx.entity.id)
                .await?;

            for rel in entity_rels {
                // Only include if both ends are in our selected set
                if entity_ids.contains(rel.source_entity_id.as_str())
                    && entity_ids.contains(rel.target_entity_id.as_str())
                {
                    // Find entity names
                    let source_name = entities
                        .iter()
                        .find(|e| e.entity.id == rel.source_entity_id)
                        .map(|e| e.entity.name.clone())
                        .unwrap_or_else(|| rel.source_entity_id.clone());

                    let target_name = entities
                        .iter()
                        .find(|e| e.entity.id == rel.target_entity_id)
                        .map(|e| e.entity.name.clone())
                        .unwrap_or_else(|| rel.target_entity_id.clone());

                    relationships.push(RelationshipContext {
                        source_name,
                        target_name,
                        relationship_type: rel.relationship_type,
                        description: format_relationship_description(&rel),
                    });
                }
            }
        }

        // Deduplicate relationships
        let mut seen = HashSet::new();
        relationships.retain(|r| {
            let key = format!(
                "{}-{}-{:?}",
                r.source_name, r.target_name, r.relationship_type
            );
            seen.insert(key)
        });

        Ok(relationships)
    }

    /// Format context for prompt injection
    fn format_context(
        &self,
        entities: &[EntityContext],
        relationships: &[RelationshipContext],
    ) -> String {
        if entities.is_empty() {
            return String::new();
        }

        let mut lines = Vec::new();

        lines.push("## Related Knowledge".to_string());
        lines.push(String::new());

        // Group entities by type
        let mut by_type: HashMap<EntityType, Vec<&EntityContext>> = HashMap::new();
        for entity in entities {
            by_type
                .entry(entity.entity.entity_type)
                .or_default()
                .push(entity);
        }

        // Format each group
        for entity_type in &self.config.priority_types {
            if let Some(type_entities) = by_type.get(entity_type) {
                lines.push(format!("### {}", pluralize_entity_type(*entity_type)));

                for entity_ctx in type_entities {
                    let entity = &entity_ctx.entity;
                    let mut entity_line = format!("- **{}**", entity.name);

                    if let Some(desc) = &entity.description {
                        entity_line.push_str(&format!(": {}", desc));
                    }

                    lines.push(entity_line);
                }
                lines.push(String::new());
            }
        }

        // Add any remaining types not in priority list
        for (entity_type, type_entities) in &by_type {
            if !self.config.priority_types.contains(entity_type) {
                lines.push(format!("### {}", pluralize_entity_type(*entity_type)));

                for entity_ctx in type_entities {
                    let entity = &entity_ctx.entity;
                    let mut entity_line = format!("- **{}**", entity.name);

                    if let Some(desc) = &entity.description {
                        entity_line.push_str(&format!(": {}", desc));
                    }

                    lines.push(entity_line);
                }
                lines.push(String::new());
            }
        }

        // Add relationships section if enabled
        if self.config.include_relationships && !relationships.is_empty() {
            lines.push("### Relationships".to_string());
            for rel in relationships {
                lines.push(format!(
                    "- {} {} {}",
                    rel.source_name,
                    format_relationship_type(rel.relationship_type),
                    rel.target_name
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

/// Format a relationship type as a verb phrase
fn format_relationship_type(rel_type: RelationshipType) -> &'static str {
    match rel_type {
        RelationshipType::Uses => "uses",
        RelationshipType::UsedBy => "is used by",
        RelationshipType::DependsOn => "depends on",
        RelationshipType::DependencyOf => "is a dependency of",
        RelationshipType::SimilarTo => "is similar to",
        RelationshipType::PrerequisiteFor => "is a prerequisite for",
        RelationshipType::Requires => "requires",
        RelationshipType::AppliesTo => "applies to",
        RelationshipType::PartOf => "is part of",
        RelationshipType::Contains => "contains",
        RelationshipType::ImplementedBy => "is implemented by",
        RelationshipType::Implements => "implements",
        RelationshipType::ConflictsWith => "conflicts with",
        RelationshipType::RelatedTo => "is related to",
    }
}

/// Format a relationship description
fn format_relationship_description(rel: &KnowledgeRelationship) -> String {
    if let Some(evidence) = rel.evidence.first() {
        evidence.description.clone()
    } else {
        format!("{:?} relationship", rel.relationship_type)
    }
}

/// Pluralize entity type for section headers
fn pluralize_entity_type(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Concept => "Concepts",
        EntityType::Technique => "Techniques",
        EntityType::Library => "Libraries",
        EntityType::Framework => "Frameworks",
        EntityType::Pattern => "Patterns",
        EntityType::Language => "Languages",
        EntityType::Tool => "Tools",
        EntityType::Domain => "Domains",
        EntityType::Api => "APIs",
        EntityType::DataStructure => "Data Structures",
        EntityType::Algorithm => "Algorithms",
    }
}

/// Estimate token count (rough approximation: ~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrichment_config_default() {
        let config = EnrichmentConfig::default();
        assert_eq!(config.max_entities, 10);
        assert_eq!(config.max_depth, 2);
        assert!(config.include_relationships);
    }

    #[test]
    fn test_enrichment_config_minimal() {
        let config = EnrichmentConfig::minimal();
        assert_eq!(config.max_entities, 5);
        assert_eq!(config.max_depth, 1);
        assert!(!config.include_relationships);
    }

    #[test]
    fn test_format_relationship_type() {
        assert_eq!(format_relationship_type(RelationshipType::Uses), "uses");
        assert_eq!(
            format_relationship_type(RelationshipType::DependsOn),
            "depends on"
        );
        assert_eq!(
            format_relationship_type(RelationshipType::SimilarTo),
            "is similar to"
        );
    }

    #[test]
    fn test_pluralize_entity_type() {
        assert_eq!(pluralize_entity_type(EntityType::Library), "Libraries");
        assert_eq!(pluralize_entity_type(EntityType::Framework), "Frameworks");
        assert_eq!(pluralize_entity_type(EntityType::Concept), "Concepts");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello world"), 2); // 11 chars / 4 ≈ 2
        assert_eq!(estimate_tokens("this is a longer text for testing"), 8);
    }

    #[test]
    fn test_enriched_context_empty() {
        let context = EnrichedContext {
            entities: Vec::new(),
            relationships: Vec::new(),
            formatted_context: String::new(),
            matched_terms: Vec::new(),
            stats: EnrichmentStats::default(),
        };

        assert!(context.entities.is_empty());
        assert!(context.formatted_context.is_empty());
    }
}
