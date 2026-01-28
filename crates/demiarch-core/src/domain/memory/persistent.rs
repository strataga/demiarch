use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;

use super::{
    Embeddings, MemoryError, MemoryRecord, MemoryStats, MemoryStore, RecallQuery, SimpleEmbedder,
    TimelineEntry,
};

/// Persistent backing store for progressive disclosure context.
///
/// Records are written to SQLite (context_entries table) and loaded into an
/// in-memory `MemoryStore` for similarity search. This keeps queries fast while
/// ensuring chat/agent context is durable between CLI runs.
#[derive(Clone)]
pub struct PersistentMemoryStore {
    pool: SqlitePool,
    embedder: Arc<dyn super::Embedder + Send + Sync>,
    embedding_model: String,
}

impl PersistentMemoryStore {
    /// Create a new persistent store using the default deterministic embedder.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            embedder: Arc::new(SimpleEmbedder::default()),
            embedding_model: "context-embedder".to_string(),
        }
    }

    /// Create a store with a custom embedder/model (useful for tests).
    pub fn with_embedder(
        pool: SqlitePool,
        embedder: Arc<dyn super::Embedder + Send + Sync>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            pool,
            embedder,
            embedding_model: model.into(),
        }
    }

    /// Ingest raw content into the store, producing a progressive summary.
    pub async fn ingest(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        source: &str,
        source_reference: Option<&str>,
        content: &str,
    ) -> Result<MemoryRecord, MemoryError> {
        let record = MemoryRecord::new(content, &self.embedding_model, self.embedder.as_ref())?;
        let highlight_json = serde_json::to_string(&record.timeline_entry.highlights)
            .map_err(|e| MemoryError::Storage(format!("Failed to serialize highlights: {e}")))?;
        let embedding_json = serde_json::to_string(&record.embeddings.index)
            .map_err(|e| MemoryError::Storage(format!("Failed to serialize embeddings: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO context_entries (
                id, project_id, conversation_id, source, source_reference,
                index_summary, timeline_summary, highlights, full_context,
                embedding_model, embedding_json, tokens_estimated,
                created_at, updated_at
            ) VALUES (
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?
            )
            ON CONFLICT(id) DO UPDATE SET
                index_summary = excluded.index_summary,
                timeline_summary = excluded.timeline_summary,
                highlights = excluded.highlights,
                full_context = excluded.full_context,
                embedding_model = excluded.embedding_model,
                embedding_json = excluded.embedding_json,
                tokens_estimated = excluded.tokens_estimated,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(record.id.to_string())
        .bind(project_id)
        .bind(conversation_id)
        .bind(source)
        .bind(source_reference)
        .bind(&record.index_summary)
        .bind(&record.timeline_entry.summary)
        .bind(highlight_json)
        .bind(&record.full_context)
        .bind(&record.embeddings.model)
        .bind(embedding_json)
        .bind(record.token_estimate() as i32)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Failed to insert context entry: {e}")))?;

        Ok(record)
    }

    /// Retrieve matching context ordered by similarity.
    pub async fn recall(
        &self,
        project_id: Option<&str>,
        query: RecallQuery,
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let mut rows: Vec<ContextEntryRow> = if let Some(pid) = project_id {
            sqlx::query_as(
                r#"
                SELECT * FROM context_entries
                WHERE project_id = ?
                ORDER BY created_at DESC
                LIMIT 500
                "#,
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT * FROM context_entries
                ORDER BY created_at DESC
                LIMIT 500
                "#,
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| MemoryError::Storage(format!("Failed to load context entries: {e}")))?;

        let store = MemoryStore::new(self.embedder.clone(), &self.embedding_model);
        for row in rows.drain(..) {
            if let Ok(record) = row.into_memory_record() {
                store.insert_record(record).await;
            }
        }

        store.recall(query).await
    }

    /// Delete entries older than the cutoff. Returns number removed.
    pub async fn prune(
        &self,
        project_id: Option<&str>,
        cutoff: DateTime<Utc>,
        dry_run: bool,
    ) -> Result<usize, MemoryError> {
        let count: (i64,) = if let Some(pid) = project_id {
            sqlx::query_as(
                "SELECT COUNT(*) FROM context_entries WHERE project_id = ? AND created_at < ?",
            )
            .bind(pid)
            .bind(cutoff)
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM context_entries WHERE created_at < ?")
                .bind(cutoff)
                .fetch_one(&self.pool)
                .await
        }
        .map_err(|e| MemoryError::Storage(format!("Failed to count context rows: {e}")))?;

        if dry_run || count.0 == 0 {
            return Ok(count.0 as usize);
        }

        let deleted = if let Some(pid) = project_id {
            sqlx::query("DELETE FROM context_entries WHERE project_id = ? AND created_at < ?")
                .bind(pid)
                .bind(cutoff)
                .execute(&self.pool)
                .await
        } else {
            sqlx::query("DELETE FROM context_entries WHERE created_at < ?")
                .bind(cutoff)
                .execute(&self.pool)
                .await
        }
        .map_err(|e| MemoryError::Storage(format!("Failed to prune context: {e}")))?;

        Ok(deleted.rows_affected() as usize)
    }

    /// Recompute summaries/embeddings from the stored full_context.
    pub async fn rebuild(&self, project_id: Option<&str>) -> Result<usize, MemoryError> {
        let rows: Vec<ContextEntryRow> = if let Some(pid) = project_id {
            sqlx::query_as("SELECT * FROM context_entries WHERE project_id = ?")
                .bind(pid)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as("SELECT * FROM context_entries")
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| MemoryError::Storage(format!("Failed to load context rows: {e}")))?;

        let mut updated = 0usize;
        for row in rows {
            let refreshed = MemoryRecord::new(
                &row.full_context,
                &self.embedding_model,
                self.embedder.as_ref(),
            )?;
            let highlight_json = serde_json::to_string(&refreshed.timeline_entry.highlights)
                .map_err(|e| {
                    MemoryError::Storage(format!("Failed to serialize highlights: {e}"))
                })?;
            let embedding_json =
                serde_json::to_string(&refreshed.embeddings.index).map_err(|e| {
                    MemoryError::Storage(format!("Failed to serialize embeddings: {e}"))
                })?;

            sqlx::query(
                r#"
                UPDATE context_entries SET
                    index_summary = ?,
                    timeline_summary = ?,
                    highlights = ?,
                    embedding_model = ?,
                    embedding_json = ?,
                    tokens_estimated = ?,
                    updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&refreshed.index_summary)
            .bind(&refreshed.timeline_entry.summary)
            .bind(highlight_json)
            .bind(&refreshed.embeddings.model)
            .bind(embedding_json)
            .bind(refreshed.token_estimate() as i32)
            .bind(Utc::now())
            .bind(row.id)
            .execute(&self.pool)
            .await
            .map_err(|e| MemoryError::Storage(format!("Failed to update context entry: {e}")))?;

            updated += 1;
        }

        Ok(updated)
    }

    /// Aggregate statistics for the stored context.
    pub async fn stats(&self, project_id: Option<&str>) -> Result<ContextStats, MemoryError> {
        let row: (i64, i64, Option<DateTime<Utc>>, Option<DateTime<Utc>>) =
            if let Some(pid) = project_id {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(*) as total, COALESCE(SUM(tokens_estimated),0) as tokens,
                           MIN(created_at) as oldest, MAX(created_at) as newest
                    FROM context_entries WHERE project_id = ?
                    "#,
                )
                .bind(pid)
                .fetch_one(&self.pool)
                .await
            } else {
                sqlx::query_as(
                    r#"
                    SELECT COUNT(*) as total, COALESCE(SUM(tokens_estimated),0) as tokens,
                           MIN(created_at) as oldest, MAX(created_at) as newest
                    FROM context_entries
                    "#,
                )
                .fetch_one(&self.pool)
                .await
            }
            .map_err(|e| MemoryError::Storage(format!("Failed to fetch context stats: {e}")))?;

        Ok(ContextStats {
            stats: MemoryStats {
                total_records: row.0 as usize,
                newest_at: row.3,
                oldest_at: row.2,
            },
            total_tokens: row.1 as usize,
        })
    }

    /// Internal helper to expose pool for tests
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

/// Lightweight stats wrapper with token totals.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextStats {
    pub stats: MemoryStats,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct ContextEntryRow {
    id: String,
    project_id: String,
    conversation_id: Option<String>,
    source: String,
    source_reference: Option<String>,
    index_summary: String,
    timeline_summary: String,
    highlights: Option<String>,
    full_context: String,
    embedding_model: String,
    embedding_json: String,
    tokens_estimated: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ContextEntryRow {
    fn into_memory_record(self) -> Result<MemoryRecord, MemoryError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| MemoryError::invalid(format!("Invalid context ID '{}': {e}", self.id)))?;

        let highlights: Vec<String> = self
            .highlights
            .as_deref()
            .map(|h| serde_json::from_str(h).unwrap_or_default())
            .unwrap_or_default();

        let timeline_entry = TimelineEntry {
            summary: self.timeline_summary,
            highlights,
            created_at: self.created_at,
        };

        let embedding_vec: Vec<f32> = serde_json::from_str(&self.embedding_json).map_err(|e| {
            MemoryError::Storage(format!(
                "Failed to parse embedding JSON for {}: {e}",
                self.id
            ))
        })?;

        let embeddings = Embeddings::new(
            &self.embedding_model,
            embedding_vec.clone(),
            embedding_vec.clone(),
            embedding_vec,
        );

        Ok(MemoryRecord {
            id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            index_summary: self.index_summary,
            timeline_entry,
            full_context: self.full_context,
            embeddings,
        })
    }
}
