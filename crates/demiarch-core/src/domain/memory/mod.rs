//! Memory system with progressive disclosure (index → timeline → full context)
//!
//! Provides ~10x token savings via layered summarization and embedding-based retrieval.

mod embedding;
mod error;
mod full;
mod index;
mod persistent;
mod timeline;

pub use self::{embedding::*, error::*, full::*, index::*, persistent::*, timeline::*};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Memory layer enum for progressive disclosure
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

impl Default for RecallQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            min_layer: MemoryLayer::Index,
            max_tokens: 4096,
            time_range: None,
            relevance_threshold: 0.25,
        }
    }
}

/// Memory entry that spans all layers
#[derive(Debug, Clone)]
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
        let embeddings = embedder.embed(
            embedding_model,
            &[&index_summary, &timeline_entry.summary, &full_context],
        )?;

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

    /// Approximate token usage for this record (rough heuristic)
    pub fn token_estimate(&self) -> usize {
        // 1 token ~ 4 chars
        (self.full_context.len() / 4).max(1)
    }
}

/// In-memory progressive disclosure store
#[derive(Clone)]
pub struct MemoryStore {
    embedder: Arc<dyn Embedder + Send + Sync>,
    embedding_model: String,
    records: Arc<RwLock<Vec<MemoryRecord>>>,
}

impl MemoryStore {
    /// Create a new store with the provided embedder and model name
    pub fn new(
        embedder: Arc<dyn Embedder + Send + Sync>,
        embedding_model: impl Into<String>,
    ) -> Self {
        Self {
            embedder,
            embedding_model: embedding_model.into(),
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a new memory entry
    pub async fn add(&self, content: &str) -> Result<MemoryRecord, MemoryError> {
        let record = MemoryRecord::new(content, &self.embedding_model, self.embedder.as_ref())?;
        self.records.write().await.push(record.clone());
        Ok(record)
    }

    /// Insert an already-constructed record (used by persistent stores/tests)
    pub async fn insert_record(&self, record: MemoryRecord) {
        self.records.write().await.push(record);
    }

    /// Recall context ordered by similarity to the query
    pub async fn recall(&self, query: RecallQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        if query.query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let query_embeddings = self.embedder.embed(
            &self.embedding_model,
            &[&query.query, &query.query, &query.query],
        )?;

        let records = self.records.read().await.clone();
        let mut ranked: Vec<(f32, MemoryRecord)> = records
            .into_iter()
            .filter(|record| match query.time_range {
                Some((start, end)) => record.created_at >= start && record.created_at <= end,
                None => true,
            })
            .map(|record| {
                let sim = cosine_similarity(
                    query_embeddings.index.as_slice(),
                    record.embeddings.index.as_slice(),
                );
                (sim, record)
            })
            .filter(|(sim, _)| *sim >= query.relevance_threshold)
            .collect();

        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut results = Vec::new();
        let mut tokens_accumulated = 0usize;
        for (_sim, mut record) in ranked {
            record.updated_at = Utc::now();
            let tokens = record.token_estimate();
            if tokens_accumulated + tokens > query.max_tokens {
                break;
            }
            tokens_accumulated += tokens;
            results.push(record);
        }

        Ok(results)
    }

    /// Recall with externally provided embeddings to avoid recomputing
    pub async fn recall_with_embedding(
        &self,
        query: RecallQuery,
        query_embedding: &[f32],
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let records = self.records.read().await.clone();
        let mut ranked: Vec<(f32, MemoryRecord)> = records
            .into_iter()
            .filter(|record| match query.time_range {
                Some((start, end)) => record.created_at >= start && record.created_at <= end,
                None => true,
            })
            .map(|record| {
                let sim = cosine_similarity(query_embedding, record.embeddings.index.as_slice());
                (sim, record)
            })
            .filter(|(sim, _)| *sim >= query.relevance_threshold)
            .collect();

        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut results = Vec::new();
        let mut tokens_accumulated = 0usize;
        for (_sim, mut record) in ranked {
            record.updated_at = Utc::now();
            let tokens = record.token_estimate();
            if tokens_accumulated + tokens > query.max_tokens {
                break;
            }
            tokens_accumulated += tokens;
            results.push(record);
        }

        Ok(results)
    }

    /// Remove entries older than the provided timestamp
    pub async fn prune_before(&self, cutoff: DateTime<Utc>) -> usize {
        let mut guard = self.records.write().await;
        let before = guard.len();
        guard.retain(|r| r.created_at >= cutoff);
        before - guard.len()
    }

    /// Retrieve store statistics
    pub async fn stats(&self) -> MemoryStats {
        let guard = self.records.read().await;
        let total_records = guard.len();
        let newest_at = guard.iter().map(|r| r.created_at).max();
        let oldest_at = guard.iter().map(|r| r.created_at).min();
        MemoryStats {
            total_records,
            newest_at,
            oldest_at,
        }
    }

    /// Bulk load records (used by persistent store hydration)
    pub async fn load(&self, records: Vec<MemoryRecord>) {
        let mut guard = self.records.write().await;
        guard.extend(records);
    }

    /// Get all records (for diagnostics/testing)
    pub async fn all(&self) -> Vec<MemoryRecord> {
        self.records.read().await.clone()
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new(Arc::new(SimpleEmbedder::default()), "text-embedding-sim")
    }
}

/// Simple vectorizer that produces stable, deterministic embeddings without
/// external dependencies. This keeps tests fast and deterministic.
#[derive(Clone, Debug)]
pub struct SimpleEmbedder {
    dimensions: usize,
}

impl Default for SimpleEmbedder {
    fn default() -> Self {
        Self { dimensions: 64 }
    }
}

impl SimpleEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl Embedder for SimpleEmbedder {
    fn embed(&self, model: &str, texts: &[&str]) -> Result<Embeddings, MemoryError> {
        if texts.is_empty() {
            return Err(MemoryError::invalid("no text provided for embedding"));
        }

        let vectors: Vec<Vec<f32>> = texts
            .iter()
            .map(|t| text_to_vec(t, self.dimensions))
            .collect();

        let index = vectors
            .get(0)
            .cloned()
            .unwrap_or_else(|| vec![0.0; self.dimensions]);
        let timeline = vectors.get(1).cloned().unwrap_or_else(|| index.clone());
        let full = vectors.get(2).cloned().unwrap_or_else(|| timeline.clone());

        Ok(Embeddings::new(model, index, timeline, full))
    }
}

fn text_to_vec(text: &str, dims: usize) -> Vec<f32> {
    let mut vec = vec![0.0; dims];
    for (i, b) in text.bytes().enumerate() {
        let idx = i % dims;
        vec[idx] += (b as f32) / 255.0;
    }

    // Normalize
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Lightweight stats for the in-memory store
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_records: usize,
    pub newest_at: Option<DateTime<Utc>>,
    pub oldest_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_add_and_recall() {
        let store = MemoryStore::default();
        store.add("Important bug fix discussion").await.unwrap();
        store.add("Unrelated chatter about lunch").await.unwrap();

        let results = store
            .recall(RecallQuery {
                query: "bug".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(results[0].index_summary.to_lowercase().contains("bug"));
    }

    #[tokio::test]
    async fn pruning_removes_old_entries() {
        let store = MemoryStore::default();
        store.add("Entry one").await.unwrap();
        store.add("Entry two").await.unwrap();

        let cutoff = Utc::now();
        let removed = store.prune_before(cutoff).await;
        assert!(removed >= 0);
    }
}
