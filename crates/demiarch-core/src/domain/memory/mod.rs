//! Memory system with progressive disclosure (index → timeline → full context)
//! 
//! Provides ~10x token savings via layered summarization and embedding-based retrieval

mod embedding;
mod index;
mod timeline;
mod full;
mod error;

pub use self::{
    embedding::*,
    error::*,
    index::*,
    timeline::*,
    full::*,
};

use crate::domain::locking::{ResourceLockGuard, ResourceType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Memory layer enum for progressive disclosure
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MemoryLayer {
    Index,
    Timeline,
    Full,
}

/// Memory recall request specifying disclosure boundaries
#[derive(Debug, Clone)]
pub struct RecallQuery {
    pub query: String,
    pub min_layer: MemoryLayer,
    pub max_tokens: usize,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub relevance_threshold: f32,
}

/// Memory entry that spans all layers
pub struct MemoryRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub index_summary: String,
    pub timeline_entry: TimelineEntry,
    pub full_context: String,
    pub embeddings: Embeddings,
}

impl MemoryRecord {
    /// Create new memory with progressive summarization
    pub fn new(
        content: &str,
        embedding_model: &str,
        embedder: &dyn Embedder,
    ) -> Result<Self, MemoryError> {
        // Generate the hierarchy of representations
        let full_context = content.to_string();
        let timeline_entry = TimelineEntry::from_content(&full_context)?;
        let index_summary = generate_index_summary(&timeline_entry)?;
        let embeddings = embedder.embed(embedding_model, &[&index_summary, &timeline_entry.summary, &full_context])?;

        Ok(Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            index_summary,
            timeline_entry,
            full_context,
            embeddings,
        })
    }
}
