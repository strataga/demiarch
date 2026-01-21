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
//! ```

mod extractor;
mod store;
mod types;

pub use extractor::{ExtractionContext, SkillExtractor};
pub use store::{SkillStats, SkillStore};
pub use types::{
    LearnedSkill, PatternType, PatternVariable, SkillCategory, SkillConfidence, SkillMetadata,
    SkillPattern, SkillSource, SkillUsageStats,
};
