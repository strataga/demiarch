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

use crate::agents::traits::AgentResult;
use crate::domain::memory::{Embedder, SimpleEmbedder};
use crate::error::{Error, Result};
use crate::llm::LlmClient;
use crate::storage::Database;
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::Arc;

/// High-level manager that wires together skill extraction and storage.
///
/// The manager is intentionally lightweight so it can be constructed in tests
/// without external dependencies. When no database is provided it falls back to
/// the default global database.
#[derive(Clone)]
pub struct SkillsManager {
    db: Option<Database>,
}

impl SkillsManager {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_database(db: Database) -> Self {
        Self { db: Some(db) }
    }

    async fn store(&self) -> Result<SkillStore> {
        if let Some(db) = &self.db {
            Ok(SkillStore::new(db.pool().clone()))
        } else {
            Err(Error::ConfigError(
                "Database not configured for SkillsManager".into(),
            ))
        }
    }

    pub async fn list(
        &self,
        category: Option<SkillCategory>,
        limit: Option<u32>,
    ) -> Result<Vec<LearnedSkill>> {
        let store = self.store().await?;
        let mut skills = if let Some(cat) = category {
            store.list_by_category(cat).await?
        } else {
            store.list().await?
        };
        if let Some(limit) = limit {
            skills.truncate(limit as usize);
        }
        Ok(skills)
    }

    pub async fn get(&self, id: &str) -> Result<Option<LearnedSkill>> {
        let store = self.store().await?;
        store.get(id).await
    }

    pub async fn search(&self, query: &str) -> Result<Vec<LearnedSkill>> {
        let store = self.store().await?;
        let mut results = store.search(query).await?;

        // Semantic search using lightweight deterministic embeddings
        let embedder = SimpleEmbedder::default();
        let query_embed = embedder
            .embed("skill-search", &[query, query, query])
            .map_err(|e| Error::Validation(format!("Failed to embed query: {e}")))?;

        // Ensure embeddings exist for skills we might rank
        for skill in store.list().await? {
            self.ensure_embedding(&store, &skill, &embedder).await?;
        }

        let semantic = store
            .semantic_search(&query_embed.index, "skill-search", 20, 0.15)
            .await?;

        // Merge: semantic first (higher quality), then append FTS uniques
        let mut seen = std::collections::HashSet::new();
        let mut merged = Vec::new();
        for res in semantic {
            if seen.insert(res.skill.id.clone()) {
                merged.push(res.skill);
            }
        }
        for skill in results.drain(..) {
            if seen.insert(skill.id.clone()) {
                merged.push(skill);
            }
        }

        Ok(merged)
    }

    pub async fn save(&self, skill: &LearnedSkill) -> Result<()> {
        let store = self.store().await?;
        store.save(skill).await?;

        // Generate/update embedding for semantic search
        let embedder = SimpleEmbedder::default();
        self.ensure_embedding(&store, skill, &embedder).await
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let store = self.store().await?;
        store.delete(id).await
    }

    pub async fn stats(&self) -> Result<SkillStats> {
        let store = self.store().await?;
        store.stats().await
    }

    pub async fn record_usage(&self, skill_id: &str, success: bool) -> Result<()> {
        let store = self.store().await?;
        store.record_usage(skill_id, success).await
    }

    /// Extract skills from an agent result using the LLM-backed extractor.
    pub async fn extract_from_result(
        &self,
        llm_client: Arc<LlmClient>,
        result: &AgentResult,
        context: ExtractionContext,
    ) -> Result<Vec<LearnedSkill>> {
        let extractor = SkillExtractor::new(llm_client);
        extractor.extract_from_result(result, context).await
    }

    async fn ensure_embedding(
        &self,
        store: &SkillStore,
        skill: &LearnedSkill,
        embedder: &SimpleEmbedder,
    ) -> Result<()> {
        let text = build_embedding_text(skill);
        let text_hash = hash_text(&text);

        if store
            .has_valid_embedding(&skill.id, "skill-search", &text_hash)
            .await?
        {
            return Ok(());
        }

        let emb = embedder
            .embed("skill-search", &[&text, &text, &text])
            .map_err(|e| Error::Validation(format!("Failed to embed skill: {e}")))?;

        store
            .save_embedding(&skill.id, &emb.index, "skill-search", &text_hash)
            .await
    }
}

impl Default for SkillsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SkillsManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SkillsManager")
    }
}

fn build_embedding_text(skill: &LearnedSkill) -> String {
    let mut parts = vec![
        skill.name.clone(),
        skill.description.clone(),
        skill.pattern.template.clone(),
    ];
    if !skill.tags.is_empty() {
        parts.push(skill.tags.join(", "));
    }
    parts.join("\n")
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}
