//! Hybrid search for GraphRAG
//!
//! This module implements cognee-inspired hybrid ranking that combines:
//! - Full-text search (FTS5 BM25)
//! - Semantic similarity (embedding cosine distance)
//! - Graph centrality (connection-based relevance)
//! - Usage signals (times_used, success_rate)

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::debug;

use crate::error::Result;
use crate::skills::{LearnedSkill, SkillStore};

use super::entity::KnowledgeEntity;
use super::repository::KnowledgeGraphRepository;
use super::relationship::RelationshipType;

/// Configuration for hybrid ranking weights
#[derive(Debug, Clone)]
pub struct HybridRankingConfig {
    /// Weight for text similarity (FTS5 BM25)
    pub text_weight: f32,
    /// Weight for embedding similarity
    pub embedding_weight: f32,
    /// Weight for graph centrality
    pub graph_weight: f32,
    /// Weight for usage statistics
    pub usage_weight: f32,
    /// Weight for entity confidence
    pub confidence_weight: f32,
    /// Decay factor per hop in graph traversal (cognee pattern)
    pub hop_decay: f32,
    /// Maximum hops for graph expansion
    pub max_hops: u32,
}

impl Default for HybridRankingConfig {
    fn default() -> Self {
        Self {
            text_weight: 0.25,
            embedding_weight: 0.30,
            graph_weight: 0.25,
            usage_weight: 0.10,
            confidence_weight: 0.10,
            hop_decay: 0.9,
            max_hops: 2,
        }
    }
}

impl HybridRankingConfig {
    /// Create a config optimized for semantic search
    pub fn semantic_focused() -> Self {
        Self {
            text_weight: 0.15,
            embedding_weight: 0.45,
            graph_weight: 0.20,
            usage_weight: 0.10,
            confidence_weight: 0.10,
            hop_decay: 0.9,
            max_hops: 2,
        }
    }

    /// Create a config optimized for graph exploration
    pub fn graph_focused() -> Self {
        Self {
            text_weight: 0.15,
            embedding_weight: 0.20,
            graph_weight: 0.45,
            usage_weight: 0.10,
            confidence_weight: 0.10,
            hop_decay: 0.85,
            max_hops: 3,
        }
    }

    /// Create a config optimized for keyword search
    pub fn keyword_focused() -> Self {
        Self {
            text_weight: 0.45,
            embedding_weight: 0.20,
            graph_weight: 0.15,
            usage_weight: 0.10,
            confidence_weight: 0.10,
            hop_decay: 0.9,
            max_hops: 1,
        }
    }
}

/// Query for graph-enhanced skill search
#[derive(Debug, Clone)]
pub struct GraphSearchQuery {
    /// Text query for FTS
    pub text_query: String,
    /// Optional embedding for semantic search
    pub embedding: Option<Vec<f32>>,
    /// Embedding model used
    pub embedding_model: Option<String>,
    /// Optional entity IDs to anchor the search
    pub anchor_entities: Vec<String>,
    /// Maximum results to return
    pub limit: usize,
    /// Minimum combined score threshold
    pub min_score: f32,
    /// Ranking configuration
    pub config: HybridRankingConfig,
}

impl GraphSearchQuery {
    /// Create a new search query
    pub fn new(text_query: impl Into<String>) -> Self {
        Self {
            text_query: text_query.into(),
            embedding: None,
            embedding_model: None,
            anchor_entities: Vec::new(),
            limit: 10,
            min_score: 0.0,
            config: HybridRankingConfig::default(),
        }
    }

    /// Add embedding for semantic search
    pub fn with_embedding(mut self, embedding: Vec<f32>, model: impl Into<String>) -> Self {
        self.embedding = Some(embedding);
        self.embedding_model = Some(model.into());
        self
    }

    /// Add anchor entities to expand from
    pub fn with_anchor_entities(mut self, entity_ids: Vec<String>) -> Self {
        self.anchor_entities = entity_ids;
        self
    }

    /// Set the result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum score threshold
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score;
        self
    }

    /// Use a specific ranking configuration
    pub fn with_config(mut self, config: HybridRankingConfig) -> Self {
        self.config = config;
        self
    }
}

/// Result from hybrid search
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// The matched skill
    pub skill: LearnedSkill,
    /// Combined score (0.0 to 1.0)
    pub score: f32,
    /// Individual score components
    pub score_breakdown: ScoreBreakdown,
    /// Related entities that contributed to the match
    pub related_entities: Vec<RelatedEntityMatch>,
    /// Number of graph hops from anchor entities (if any)
    pub graph_distance: Option<u32>,
}

/// Breakdown of how the score was computed
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    /// Text similarity score (FTS5)
    pub text_score: f32,
    /// Embedding similarity score
    pub embedding_score: f32,
    /// Graph centrality score
    pub graph_score: f32,
    /// Usage statistics score
    pub usage_score: f32,
    /// Confidence score
    pub confidence_score: f32,
}

impl ScoreBreakdown {
    /// Compute weighted combined score
    pub fn combined(&self, config: &HybridRankingConfig) -> f32 {
        self.text_score * config.text_weight
            + self.embedding_score * config.embedding_weight
            + self.graph_score * config.graph_weight
            + self.usage_score * config.usage_weight
            + self.confidence_score * config.confidence_weight
    }
}

/// Entity that contributed to a match
#[derive(Debug, Clone)]
pub struct RelatedEntityMatch {
    /// The entity
    pub entity: KnowledgeEntity,
    /// How it relates to the query
    pub relevance: EntityRelevance,
    /// Contribution to the score
    pub contribution: f32,
}

/// How an entity is relevant to the query
#[derive(Debug, Clone)]
pub enum EntityRelevance {
    /// Direct text match
    DirectMatch,
    /// Semantic similarity
    SemanticMatch { similarity: f32 },
    /// Connected via graph relationship
    GraphConnected {
        relationship_type: RelationshipType,
        hops: u32,
    },
    /// Anchor entity
    Anchor,
}

/// Hybrid ranker for combining multiple signals
pub struct HybridRanker<R: KnowledgeGraphRepository> {
    /// Knowledge graph repository
    repository: Arc<R>,
    /// Skill store for FTS and embeddings
    skill_store: Arc<SkillStore>,
}

impl<R: KnowledgeGraphRepository> HybridRanker<R> {
    /// Create a new hybrid ranker
    pub fn new(repository: Arc<R>, skill_store: Arc<SkillStore>) -> Self {
        Self {
            repository,
            skill_store,
        }
    }

    /// Execute a hybrid search
    pub async fn search(&self, query: &GraphSearchQuery) -> Result<Vec<HybridSearchResult>> {
        let config = &query.config;

        // Step 1: Get candidate skills from multiple sources
        let mut candidate_skills = self.get_candidate_skills(query).await?;

        // Step 2: Get entity matches and graph context
        let entity_matches = self.get_entity_matches(query).await?;

        // Step 3: Build skill-to-entity relevance map
        let skill_entity_map = self.build_skill_entity_map(&candidate_skills, &entity_matches).await?;

        // Step 4: Score each candidate
        let mut results = Vec::new();

        for skill in candidate_skills.drain(..) {
            let score_breakdown = self
                .compute_score(&skill, query, &entity_matches, &skill_entity_map)
                .await?;

            let combined_score = score_breakdown.combined(config);

            // Apply hop penalty if graph distance is known
            let (final_score, graph_distance) = if let Some(entities) = skill_entity_map.get(&skill.id) {
                let min_distance = entities
                    .iter()
                    .filter_map(|(_, dist)| *dist)
                    .min()
                    .unwrap_or(0);
                let hop_penalty = config.hop_decay.powi(min_distance as i32);
                (combined_score * hop_penalty, Some(min_distance))
            } else {
                (combined_score, None)
            };

            if final_score >= query.min_score {
                // Get related entities for this skill
                let related_entities = self
                    .get_related_entities_for_skill(&skill, &entity_matches, &skill_entity_map)
                    .await?;

                results.push(HybridSearchResult {
                    skill,
                    score: final_score,
                    score_breakdown,
                    related_entities,
                    graph_distance,
                });
            }
        }

        // Step 5: Sort by score and limit
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(query.limit);

        debug!(
            query = %query.text_query,
            results = results.len(),
            "Hybrid search completed"
        );

        Ok(results)
    }

    /// Get candidate skills from FTS and semantic search
    async fn get_candidate_skills(&self, query: &GraphSearchQuery) -> Result<Vec<LearnedSkill>> {
        let mut candidates = HashSet::new();
        let mut skills = Vec::new();

        // FTS search
        let fts_results = self.skill_store.search(&query.text_query).await?;
        for skill in fts_results {
            if candidates.insert(skill.id.clone()) {
                skills.push(skill);
            }
        }

        // Semantic search if embedding provided
        if let (Some(embedding), Some(model)) = (&query.embedding, &query.embedding_model) {
            let semantic_results = self
                .skill_store
                .semantic_search(embedding, model, query.limit * 2, 0.3)
                .await?;
            for result in semantic_results {
                if candidates.insert(result.skill.id.clone()) {
                    skills.push(result.skill);
                }
            }
        }

        // Skills linked to anchor entities
        for entity_id in &query.anchor_entities {
            let linked_skills = self.repository.get_skills_for_entity(entity_id).await?;
            for skill_id in linked_skills {
                if candidates.insert(skill_id.clone()) {
                    if let Some(skill) = self.skill_store.get(&skill_id).await? {
                        skills.push(skill);
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Get entity matches from text and semantic search
    async fn get_entity_matches(
        &self,
        query: &GraphSearchQuery,
    ) -> Result<HashMap<String, (KnowledgeEntity, f32)>> {
        let mut matches = HashMap::new();

        // Text search on entities
        let text_matches = self
            .repository
            .search_entities(&query.text_query, query.limit * 2)
            .await?;

        for entity in text_matches {
            matches.insert(entity.id.clone(), (entity, 1.0)); // FTS match gets score 1.0
        }

        // Semantic search if embedding provided
        if let (Some(embedding), Some(model)) = (&query.embedding, &query.embedding_model) {
            let semantic_matches = self
                .repository
                .semantic_search_entities(embedding, model, query.limit * 2, 0.3)
                .await?;

            for result in semantic_matches {
                matches
                    .entry(result.entity.id.clone())
                    .and_modify(|(_, score)| *score = f32::max(*score, result.similarity))
                    .or_insert((result.entity, result.similarity));
            }
        }

        // Add anchor entities
        for entity_id in &query.anchor_entities {
            if let Some(entity) = self.repository.get_entity(entity_id).await? {
                matches.insert(entity.id.clone(), (entity, 1.0));
            }
        }

        // Expand graph from matched entities
        let config = &query.config;
        if config.max_hops > 0 && !matches.is_empty() {
            let entity_ids: Vec<String> = matches.keys().cloned().collect();
            for entity_id in entity_ids {
                let neighbors = self
                    .repository
                    .get_neighborhood(&entity_id, config.max_hops, None)
                    .await?;

                for neighbor in neighbors {
                    let decay = config.hop_decay.powi(neighbor.distance as i32);
                    matches
                        .entry(neighbor.entity.id.clone())
                        .and_modify(|(_, score)| *score = score.max(decay))
                        .or_insert((neighbor.entity, decay));
                }
            }
        }

        Ok(matches)
    }

    /// Build map from skill ID to linked entities with their distance
    async fn build_skill_entity_map(
        &self,
        skills: &[LearnedSkill],
        entity_matches: &HashMap<String, (KnowledgeEntity, f32)>,
    ) -> Result<HashMap<String, Vec<(String, Option<u32>)>>> {
        let mut map = HashMap::new();

        for skill in skills {
            let linked_entities = self.repository.get_entities_for_skill(&skill.id).await?;
            let mut skill_entities = Vec::new();

            for entity in linked_entities {
                if entity_matches.contains_key(&entity.id) {
                    // Direct match
                    skill_entities.push((entity.id, Some(0)));
                } else {
                    // Check if entity is connected to a match
                    for match_id in entity_matches.keys() {
                        if let Some(path) = self.repository.find_path(&entity.id, match_id, 2).await? {
                            skill_entities.push((entity.id.clone(), Some((path.len() - 1) as u32)));
                            break;
                        }
                    }
                }
            }

            if !skill_entities.is_empty() {
                map.insert(skill.id.clone(), skill_entities);
            }
        }

        Ok(map)
    }

    /// Compute score breakdown for a skill
    async fn compute_score(
        &self,
        skill: &LearnedSkill,
        query: &GraphSearchQuery,
        entity_matches: &HashMap<String, (KnowledgeEntity, f32)>,
        skill_entity_map: &HashMap<String, Vec<(String, Option<u32>)>>,
    ) -> Result<ScoreBreakdown> {
        let mut breakdown = ScoreBreakdown::default();

        // Text score: based on whether skill was found via FTS
        // (simplified - in production would compute BM25 relevance)
        let text_match = skill.matches_query(&query.text_query);
        breakdown.text_score = if text_match { 0.8 } else { 0.0 };

        // Embedding score: if we have embeddings, compute similarity
        if let (Some(query_embedding), Some(model)) = (&query.embedding, &query.embedding_model) {
            if let Some(embedding) = self
                .skill_store
                .get_embedding(&skill.id, model)
                .await?
            {
                breakdown.embedding_score = cosine_similarity(query_embedding, &embedding.embedding);
            }
        }

        // Graph score: based on connections to matched entities
        if let Some(linked_entities) = skill_entity_map.get(&skill.id) {
            let graph_score: f32 = linked_entities
                .iter()
                .filter_map(|(entity_id, _)| entity_matches.get(entity_id).map(|(_, score)| *score))
                .sum::<f32>()
                / linked_entities.len().max(1) as f32;
            breakdown.graph_score = graph_score.min(1.0);
        }

        // Usage score: based on success rate and usage count
        let usage_score = if skill.usage_stats.times_used > 0 {
            let recency_factor = 1.0; // Could factor in last_used_at
            let success_factor = skill.usage_stats.success_rate() as f32;
            let usage_factor = (skill.usage_stats.times_used as f32).ln() / 10.0;
            ((success_factor * 0.5 + usage_factor * 0.5) * recency_factor).min(1.0)
        } else {
            0.5 // Default for unused skills
        };
        breakdown.usage_score = usage_score;

        // Confidence score: skill confidence
        breakdown.confidence_score = skill.confidence.score() as f32 / 3.0;

        Ok(breakdown)
    }

    /// Get related entities for a skill result
    async fn get_related_entities_for_skill(
        &self,
        skill: &LearnedSkill,
        entity_matches: &HashMap<String, (KnowledgeEntity, f32)>,
        skill_entity_map: &HashMap<String, Vec<(String, Option<u32>)>>,
    ) -> Result<Vec<RelatedEntityMatch>> {
        let mut related = Vec::new();

        if let Some(linked) = skill_entity_map.get(&skill.id) {
            for (entity_id, distance) in linked {
                if let Some((entity, score)) = entity_matches.get(entity_id) {
                    let relevance = if distance == &Some(0) {
                        EntityRelevance::DirectMatch
                    } else if let Some(dist) = distance {
                        EntityRelevance::GraphConnected {
                            relationship_type: RelationshipType::RelatedTo,
                            hops: *dist,
                        }
                    } else {
                        EntityRelevance::SemanticMatch { similarity: *score }
                    };

                    related.push(RelatedEntityMatch {
                        entity: entity.clone(),
                        relevance,
                        contribution: *score,
                    });
                }
            }
        }

        // Sort by contribution
        related.sort_by(|a, b| {
            b.contribution
                .partial_cmp(&a.contribution)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(related)
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_ranking_config() {
        let config = HybridRankingConfig::default();
        assert!((config.text_weight + config.embedding_weight + config.graph_weight
            + config.usage_weight + config.confidence_weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_score_breakdown_combined() {
        let config = HybridRankingConfig::default();
        let breakdown = ScoreBreakdown {
            text_score: 1.0,
            embedding_score: 1.0,
            graph_score: 1.0,
            usage_score: 1.0,
            confidence_score: 1.0,
        };

        let combined = breakdown.combined(&config);
        assert!((combined - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_graph_search_query() {
        let query = GraphSearchQuery::new("error handling")
            .with_limit(20)
            .with_min_score(0.5)
            .with_anchor_entities(vec!["entity-1".into()]);

        assert_eq!(query.text_query, "error handling");
        assert_eq!(query.limit, 20);
        assert_eq!(query.min_score, 0.5);
        assert_eq!(query.anchor_entities.len(), 1);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[test]
    fn test_semantic_focused_config() {
        let config = HybridRankingConfig::semantic_focused();
        assert!(config.embedding_weight > config.text_weight);
        assert!(config.embedding_weight > config.graph_weight);
    }

    #[test]
    fn test_graph_focused_config() {
        let config = HybridRankingConfig::graph_focused();
        assert!(config.graph_weight > config.text_weight);
        assert!(config.graph_weight > config.embedding_weight);
    }
}
