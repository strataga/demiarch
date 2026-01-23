//! Persistent storage for routing statistics
//!
//! This module provides SQLite-based persistence for model routing statistics,
//! enabling the RL system to learn across sessions.

use std::collections::HashMap;
use std::path::Path;

use sqlx::{Row, SqlitePool};
use tracing::{debug, info, warn};

use super::types::ModelStats;
use crate::error::{Error, Result};

/// SQL to create the routing statistics table
pub const CREATE_ROUTING_STATS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS routing_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    routing_key TEXT NOT NULL,
    model_id TEXT NOT NULL,
    total_uses INTEGER NOT NULL DEFAULT 0,
    successes INTEGER NOT NULL DEFAULT 0,
    failures INTEGER NOT NULL DEFAULT 0,
    reward_sum REAL NOT NULL DEFAULT 0.0,
    reward_sum_sq REAL NOT NULL DEFAULT 0.0,
    avg_cost_usd REAL NOT NULL DEFAULT 0.0,
    avg_latency_ms REAL NOT NULL DEFAULT 0.0,
    alpha REAL NOT NULL DEFAULT 1.0,
    beta REAL NOT NULL DEFAULT 1.0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(routing_key, model_id)
);

CREATE INDEX IF NOT EXISTS idx_routing_stats_key ON routing_stats(routing_key);
CREATE INDEX IF NOT EXISTS idx_routing_stats_model ON routing_stats(model_id);
"#;

/// Store for persisting routing statistics
pub struct RoutingStore {
    pool: SqlitePool,
}

impl RoutingStore {
    /// Create a new store from an existing connection pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new store and connect to the database
    pub async fn connect(database_path: &Path) -> Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", database_path.display());

        let pool = SqlitePool::connect(&url)
            .await
            .map_err(Error::DatabaseError)?;

        Ok(Self { pool })
    }

    /// Initialize the database schema
    pub async fn init(&self) -> Result<()> {
        sqlx::query(CREATE_ROUTING_STATS_TABLE_SQL)
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        info!("Routing statistics table initialized");
        Ok(())
    }

    /// Save or update statistics for a (routing_key, model) pair
    pub async fn save_stats(&self, stats: &ModelStats) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO routing_stats (
                routing_key, model_id, total_uses, successes, failures,
                reward_sum, reward_sum_sq, avg_cost_usd, avg_latency_ms,
                alpha, beta, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(routing_key, model_id) DO UPDATE SET
                total_uses = excluded.total_uses,
                successes = excluded.successes,
                failures = excluded.failures,
                reward_sum = excluded.reward_sum,
                reward_sum_sq = excluded.reward_sum_sq,
                avg_cost_usd = excluded.avg_cost_usd,
                avg_latency_ms = excluded.avg_latency_ms,
                alpha = excluded.alpha,
                beta = excluded.beta,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&stats.routing_key)
        .bind(&stats.model_id)
        .bind(stats.total_uses as i64)
        .bind(stats.successes as i64)
        .bind(stats.failures as i64)
        .bind(stats.reward_sum)
        .bind(stats.reward_sum_sq)
        .bind(stats.avg_cost_usd)
        .bind(stats.avg_latency_ms)
        .bind(stats.alpha)
        .bind(stats.beta)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        debug!(
            routing_key = %stats.routing_key,
            model_id = %stats.model_id,
            "Saved routing statistics"
        );

        Ok(())
    }

    /// Save multiple statistics atomically
    pub async fn save_all_stats(&self, stats: &[ModelStats]) -> Result<()> {
        if stats.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(Error::DatabaseError)?;

        for stat in stats {
            sqlx::query(
                r#"
                INSERT INTO routing_stats (
                    routing_key, model_id, total_uses, successes, failures,
                    reward_sum, reward_sum_sq, avg_cost_usd, avg_latency_ms,
                    alpha, beta, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(routing_key, model_id) DO UPDATE SET
                    total_uses = excluded.total_uses,
                    successes = excluded.successes,
                    failures = excluded.failures,
                    reward_sum = excluded.reward_sum,
                    reward_sum_sq = excluded.reward_sum_sq,
                    avg_cost_usd = excluded.avg_cost_usd,
                    avg_latency_ms = excluded.avg_latency_ms,
                    alpha = excluded.alpha,
                    beta = excluded.beta,
                    updated_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(&stat.routing_key)
            .bind(&stat.model_id)
            .bind(stat.total_uses as i64)
            .bind(stat.successes as i64)
            .bind(stat.failures as i64)
            .bind(stat.reward_sum)
            .bind(stat.reward_sum_sq)
            .bind(stat.avg_cost_usd)
            .bind(stat.avg_latency_ms)
            .bind(stat.alpha)
            .bind(stat.beta)
            .execute(&mut *tx)
            .await
            .map_err(Error::DatabaseError)?;
        }

        tx.commit().await.map_err(Error::DatabaseError)?;

        info!(count = stats.len(), "Saved batch routing statistics");
        Ok(())
    }

    /// Load statistics for a specific routing key
    pub async fn load_stats_for_key(
        &self,
        routing_key: &str,
    ) -> Result<HashMap<String, ModelStats>> {
        let rows = sqlx::query(
            r#"
            SELECT routing_key, model_id, total_uses, successes, failures,
                   reward_sum, reward_sum_sq, avg_cost_usd, avg_latency_ms, alpha, beta
            FROM routing_stats
            WHERE routing_key = ?
            "#,
        )
        .bind(routing_key)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        let mut result = HashMap::new();
        for row in rows {
            let stats = ModelStats {
                routing_key: row.get("routing_key"),
                model_id: row.get("model_id"),
                total_uses: row.get::<i64, _>("total_uses") as u64,
                successes: row.get::<i64, _>("successes") as u64,
                failures: row.get::<i64, _>("failures") as u64,
                reward_sum: row.get("reward_sum"),
                reward_sum_sq: row.get("reward_sum_sq"),
                avg_cost_usd: row.get("avg_cost_usd"),
                avg_latency_ms: row.get("avg_latency_ms"),
                alpha: row.get("alpha"),
                beta: row.get("beta"),
            };
            result.insert(stats.model_id.clone(), stats);
        }

        debug!(routing_key = %routing_key, count = result.len(), "Loaded routing statistics");
        Ok(result)
    }

    /// Load all statistics from the database
    pub async fn load_all_stats(&self) -> Result<HashMap<String, HashMap<String, ModelStats>>> {
        let rows = sqlx::query(
            r#"
            SELECT routing_key, model_id, total_uses, successes, failures,
                   reward_sum, reward_sum_sq, avg_cost_usd, avg_latency_ms, alpha, beta
            FROM routing_stats
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        let mut result: HashMap<String, HashMap<String, ModelStats>> = HashMap::new();

        for row in rows {
            let stats = ModelStats {
                routing_key: row.get("routing_key"),
                model_id: row.get("model_id"),
                total_uses: row.get::<i64, _>("total_uses") as u64,
                successes: row.get::<i64, _>("successes") as u64,
                failures: row.get::<i64, _>("failures") as u64,
                reward_sum: row.get("reward_sum"),
                reward_sum_sq: row.get("reward_sum_sq"),
                avg_cost_usd: row.get("avg_cost_usd"),
                avg_latency_ms: row.get("avg_latency_ms"),
                alpha: row.get("alpha"),
                beta: row.get("beta"),
            };

            result
                .entry(stats.routing_key.clone())
                .or_default()
                .insert(stats.model_id.clone(), stats);
        }

        info!(routing_keys = result.len(), "Loaded all routing statistics");
        Ok(result)
    }

    /// Delete statistics for a specific routing key
    pub async fn delete_stats_for_key(&self, routing_key: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM routing_stats WHERE routing_key = ?")
            .bind(routing_key)
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        let deleted = result.rows_affected();
        info!(routing_key = %routing_key, deleted = deleted, "Deleted routing statistics");
        Ok(deleted)
    }

    /// Delete statistics for a specific model across all routing keys
    pub async fn delete_stats_for_model(&self, model_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM routing_stats WHERE model_id = ?")
            .bind(model_id)
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        let deleted = result.rows_affected();
        info!(model_id = %model_id, deleted = deleted, "Deleted model statistics");
        Ok(deleted)
    }

    /// Clear all statistics (useful for testing or reset)
    pub async fn clear_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM routing_stats")
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        let deleted = result.rows_affected();
        warn!(deleted = deleted, "Cleared all routing statistics");
        Ok(deleted)
    }

    /// Get summary statistics across all routing keys
    pub async fn get_summary(&self) -> Result<RoutingStoreSummary> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(DISTINCT routing_key) as routing_key_count,
                COUNT(DISTINCT model_id) as model_count,
                SUM(total_uses) as total_uses,
                SUM(successes) as total_successes,
                SUM(failures) as total_failures,
                AVG(avg_cost_usd) as avg_cost,
                AVG(avg_latency_ms) as avg_latency
            FROM routing_stats
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(RoutingStoreSummary {
            routing_key_count: row.get::<i64, _>("routing_key_count") as u64,
            model_count: row.get::<i64, _>("model_count") as u64,
            total_uses: row.get::<Option<i64>, _>("total_uses").unwrap_or(0) as u64,
            total_successes: row.get::<Option<i64>, _>("total_successes").unwrap_or(0) as u64,
            total_failures: row.get::<Option<i64>, _>("total_failures").unwrap_or(0) as u64,
            avg_cost_usd: row.get::<Option<f64>, _>("avg_cost").unwrap_or(0.0),
            avg_latency_ms: row.get::<Option<f64>, _>("avg_latency").unwrap_or(0.0),
        })
    }

    /// Get the top N best-performing models for a routing key
    pub async fn get_top_models(&self, routing_key: &str, limit: u32) -> Result<Vec<ModelStats>> {
        let rows = sqlx::query(
            r#"
            SELECT routing_key, model_id, total_uses, successes, failures,
                   reward_sum, reward_sum_sq, avg_cost_usd, avg_latency_ms, alpha, beta
            FROM routing_stats
            WHERE routing_key = ?
            ORDER BY (alpha / (alpha + beta)) DESC
            LIMIT ?
            "#,
        )
        .bind(routing_key)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        let result: Vec<ModelStats> = rows
            .into_iter()
            .map(|row| ModelStats {
                routing_key: row.get("routing_key"),
                model_id: row.get("model_id"),
                total_uses: row.get::<i64, _>("total_uses") as u64,
                successes: row.get::<i64, _>("successes") as u64,
                failures: row.get::<i64, _>("failures") as u64,
                reward_sum: row.get("reward_sum"),
                reward_sum_sq: row.get("reward_sum_sq"),
                avg_cost_usd: row.get("avg_cost_usd"),
                avg_latency_ms: row.get("avg_latency_ms"),
                alpha: row.get("alpha"),
                beta: row.get("beta"),
            })
            .collect();

        Ok(result)
    }
}

/// Summary of routing store statistics
#[derive(Debug, Clone)]
pub struct RoutingStoreSummary {
    /// Number of unique routing keys
    pub routing_key_count: u64,
    /// Number of unique models tracked
    pub model_count: u64,
    /// Total number of model uses
    pub total_uses: u64,
    /// Total successful outcomes
    pub total_successes: u64,
    /// Total failed outcomes
    pub total_failures: u64,
    /// Average cost across all models
    pub avg_cost_usd: f64,
    /// Average latency across all models
    pub avg_latency_ms: f64,
}

impl RoutingStoreSummary {
    /// Get overall success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_uses == 0 {
            return 0.0;
        }
        self.total_successes as f64 / self.total_uses as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    async fn create_test_store() -> (RoutingStore, TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_routing.db");

        let store = RoutingStore::connect(&db_path).await.unwrap();
        store.init().await.unwrap();

        // Return the dir to keep it alive for the test duration
        (store, dir)
    }

    #[tokio::test]
    async fn test_save_and_load_stats() {
        let (store, _temp) = create_test_store().await;

        let stats = ModelStats::new("coder:medium".to_string(), "test/model-a".to_string());

        // Save
        store.save_stats(&stats).await.unwrap();

        // Load
        let loaded = store.load_stats_for_key("coder:medium").await.unwrap();
        assert!(loaded.contains_key("test/model-a"));

        let loaded_stats = loaded.get("test/model-a").unwrap();
        assert_eq!(loaded_stats.routing_key, "coder:medium");
        assert_eq!(loaded_stats.model_id, "test/model-a");
    }

    #[tokio::test]
    async fn test_update_existing_stats() {
        let (store, _temp) = create_test_store().await;

        let mut stats = ModelStats::new("coder:medium".to_string(), "test/model-a".to_string());

        // Save initial
        store.save_stats(&stats).await.unwrap();

        // Update
        stats.alpha = 10.0;
        stats.beta = 5.0;
        stats.total_uses = 15;
        store.save_stats(&stats).await.unwrap();

        // Verify update
        let loaded = store.load_stats_for_key("coder:medium").await.unwrap();
        let loaded_stats = loaded.get("test/model-a").unwrap();
        assert_eq!(loaded_stats.alpha, 10.0);
        assert_eq!(loaded_stats.beta, 5.0);
        assert_eq!(loaded_stats.total_uses, 15);
    }

    #[tokio::test]
    async fn test_save_batch() {
        let (store, _temp) = create_test_store().await;

        let stats = vec![
            ModelStats::new("coder:simple".to_string(), "model-a".to_string()),
            ModelStats::new("coder:simple".to_string(), "model-b".to_string()),
            ModelStats::new("planner:complex".to_string(), "model-a".to_string()),
        ];

        store.save_all_stats(&stats).await.unwrap();

        // Verify all saved
        let all = store.load_all_stats().await.unwrap();
        assert_eq!(all.len(), 2); // 2 routing keys
        assert_eq!(all.get("coder:simple").unwrap().len(), 2);
        assert_eq!(all.get("planner:complex").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_stats() {
        let (store, _temp) = create_test_store().await;

        // Save some stats
        let stats = vec![
            ModelStats::new("coder:simple".to_string(), "model-a".to_string()),
            ModelStats::new("coder:simple".to_string(), "model-b".to_string()),
        ];
        store.save_all_stats(&stats).await.unwrap();

        // Delete by routing key
        let deleted = store.delete_stats_for_key("coder:simple").await.unwrap();
        assert_eq!(deleted, 2);

        // Verify deleted
        let loaded = store.load_stats_for_key("coder:simple").await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_get_summary() {
        let (store, _temp) = create_test_store().await;

        let mut stats1 = ModelStats::new("coder:simple".to_string(), "model-a".to_string());
        stats1.total_uses = 10;
        stats1.successes = 8;
        stats1.avg_cost_usd = 0.05;

        let mut stats2 = ModelStats::new("coder:complex".to_string(), "model-b".to_string());
        stats2.total_uses = 5;
        stats2.successes = 4;
        stats2.avg_cost_usd = 0.15;

        store.save_all_stats(&[stats1, stats2]).await.unwrap();

        let summary = store.get_summary().await.unwrap();
        assert_eq!(summary.routing_key_count, 2);
        assert_eq!(summary.model_count, 2);
        assert_eq!(summary.total_uses, 15);
        assert_eq!(summary.total_successes, 12);
    }

    #[tokio::test]
    async fn test_get_top_models() {
        let (store, _temp) = create_test_store().await;

        // Create stats with different expected values
        let mut stats1 = ModelStats::new("coder:simple".to_string(), "model-a".to_string());
        stats1.alpha = 10.0; // expected = 10/11 ≈ 0.91
        stats1.beta = 1.0;

        let mut stats2 = ModelStats::new("coder:simple".to_string(), "model-b".to_string());
        stats2.alpha = 5.0; // expected = 5/10 = 0.5
        stats2.beta = 5.0;

        let mut stats3 = ModelStats::new("coder:simple".to_string(), "model-c".to_string());
        stats3.alpha = 2.0; // expected = 2/12 ≈ 0.17
        stats3.beta = 10.0;

        store
            .save_all_stats(&[stats1, stats2, stats3])
            .await
            .unwrap();

        let top = store.get_top_models("coder:simple", 2).await.unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].model_id, "model-a"); // Highest expected value
        assert_eq!(top[1].model_id, "model-b"); // Second highest
    }

    #[tokio::test]
    async fn test_clear_all() {
        let (store, _temp) = create_test_store().await;

        // Save some stats
        let stats = vec![
            ModelStats::new("key1".to_string(), "model-a".to_string()),
            ModelStats::new("key2".to_string(), "model-b".to_string()),
        ];
        store.save_all_stats(&stats).await.unwrap();

        // Clear all
        let cleared = store.clear_all().await.unwrap();
        assert_eq!(cleared, 2);

        // Verify cleared
        let all = store.load_all_stats().await.unwrap();
        assert!(all.is_empty());
    }
}
