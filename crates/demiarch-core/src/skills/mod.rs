//! Learned skills system - autonomous knowledge extraction
//!
//! This module provides the skill extraction and storage system for demiarch.
//! Skills are patterns, techniques, and reusable knowledge extracted from
//! agent interactions that can be applied to future tasks.
//!
//! # Architecture
//!
//! The skill system consists of:
//! - **LearnedSkill**: The core data model representing extracted knowledge
//! - **SkillExtractor**: LLM-assisted pattern recognition from agent results
//! - **SkillStore**: Persistent storage and retrieval of skills
//! - **SemanticSearch**: Vector-based similarity search for skill matching
//!
//! # Example
//!
//! ```rust,ignore
//! use demiarch_core::skills::{SkillExtractor, SkillStore};
//!
//! // Extract skills from an agent result
//! let extractor = SkillExtractor::new(llm_client);
//! let skills = extractor.extract_from_result(&agent_result, context).await?;
//!
//! // Store skills for future use
//! let store = SkillStore::new(db_pool);
//! for skill in skills {
//!     store.save(&skill).await?;
//! }
//!
//! // Semantic search for relevant skills
//! let query_embedding = llm_client.embed("error handling in async code", None).await?;
//! let results = store.semantic_search(&query_embedding.vector, "openai/text-embedding-3-small", 10, 0.5).await?;
//! ```

mod extractor;
mod store;
mod types;

pub use extractor::{ExtractionContext, SkillExtractor};
pub use store::{EmbeddingStats, SemanticSearchResult, SkillEmbedding, SkillStats, SkillStore};
pub use types::{
    LearnedSkill, PatternType, PatternVariable, SkillCategory, SkillConfidence, SkillMetadata,
    SkillPattern, SkillSource, SkillUsageStats,
};
