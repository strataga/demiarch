//! Knowledge graph domain module for GraphRAG
//!
//! This module implements a cognee-inspired knowledge graph system for
//! enhancing skill retrieval through:
//!
//! - **Entity extraction**: Extract concepts, techniques, libraries from skills
//! - **Relationship inference**: Build a graph of how entities relate
//! - **Graph traversal**: Multi-hop retrieval for richer context
//! - **Hybrid ranking**: Combine FTS, embeddings, and graph signals
//! - **Context enrichment**: Inject related concepts into agent prompts
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         ECL Pipeline                             │
//! │   LearnedSkill → EntityExtractor → KnowledgeGraph → Embeddings  │
//! └─────────────────────────────────────────────────────────────────┘
//!                               ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    GraphRAG Search Pipeline                      │
//! │  Query → [FTS5 + Embedding + Graph Traversal] → HybridRanker    │
//! └─────────────────────────────────────────────────────────────────┘
//!                               ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Context Enrichment                            │
//! │  Task → ContextEnricher → EnrichedContext → System Prompt       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Data Model
//!
//! - **KnowledgeEntity**: A node representing a concept, library, pattern, etc.
//! - **KnowledgeRelationship**: An edge connecting two entities
//! - **SkillEntityLink**: Connection between a skill and its entities
//!
//! ## Usage
//!
//! ```rust,ignore
//! use demiarch_core::domain::knowledge::{KnowledgeEntity, EntityType, ContextEnricher};
//!
//! // Create an entity
//! let entity = KnowledgeEntity::new("tokio", EntityType::Library)
//!     .with_description("Async runtime for Rust")
//!     .with_confidence(0.8);
//!
//! // Save to repository
//! repository.save_entity(&entity).await?;
//!
//! // Traverse the graph
//! let neighborhood = repository.get_neighborhood(&entity.id, 2, None).await?;
//!
//! // Enrich context for prompts
//! let enricher = ContextEnricher::new(repository);
//! let context = enricher.enrich_from_query("async error handling in Rust").await?;
//! println!("Related concepts: {}", context.formatted_context);
//! ```

mod enricher;
mod entity;
mod event;
mod extractor;
mod relationship;
mod repository;
mod search;
mod service;

pub use enricher::{
    ContextEnricher, EnrichedContext, EnrichmentConfig, EnrichmentStats, EntityContext,
    RelationshipContext,
};
pub use entity::{EntityType, KnowledgeEntity};
pub use event::KnowledgeEvent;
pub use extractor::{EntityExtractor, ExtractionResult};
pub use relationship::{
    EvidenceSource, KnowledgeRelationship, RelationshipEvidence, RelationshipType,
};
pub use repository::{
    EntitySearchResult, EntityWithDistance, KnowledgeGraphRepository, KnowledgeGraphStats,
    PathRelationship, PathStep, TraversalDirection,
};
pub use search::{
    EntityRelevance, GraphSearchQuery, HybridRanker, HybridRankingConfig, HybridSearchResult,
    RelatedEntityMatch, ScoreBreakdown,
};
pub use service::{CognifyResult, KnowledgeGraphService};
