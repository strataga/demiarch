//! Checkpoint repository for database operations
//!
//! Handles all database interactions for checkpoints.

use super::checkpoint::{Checkpoint, CheckpointInfo};
use super::repository_trait::{
    CheckpointRepositoryTrait, FeatureRow as TraitFeatureRow, MessageRow as TraitMessageRow,
    PhaseRow as TraitPhaseRow,
};
use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Type alias for feature database row
type FeatureRow = (
    String,         // id
    String,         // title
    Option<String>, // description
    String,         // status
    Option<String>, // phase_id
    i32,            // priority
    Option<String>, // acceptance_criteria
    Option<String>, // labels
);

/// Repository for checkpoint database operations
#[derive(Debug, Clone)]
pub struct CheckpointRepository {
    pool: SqlitePool,
}

impl CheckpointRepository {
    /// Create a new repository with the given connection pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Save a checkpoint to the database
    pub async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let id = checkpoint.id.to_string();
        let project_id = checkpoint.project_id.to_string();
        let feature_id = checkpoint.feature_id.map(|f| f.to_string());
        let snapshot_data = checkpoint.snapshot_data.to_string();

        sqlx::query(
            r#"
            INSERT INTO checkpoints (id, project_id, feature_id, description, snapshot_data, size_bytes, signature, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&project_id)
        .bind(&feature_id)
        .bind(&checkpoint.description)
        .bind(&snapshot_data)
        .bind(checkpoint.size_bytes)
        .bind(&checkpoint.signature)
        .bind(checkpoint.created_at)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    /// Get a checkpoint by ID
    pub async fn get(&self, checkpoint_id: Uuid) -> Result<Option<Checkpoint>> {
        let id = checkpoint_id.to_string();

        let row: Option<CheckpointRow> = sqlx::query_as(
            r#"
            SELECT id, project_id, feature_id, description, snapshot_data, size_bytes, signature, created_at
            FROM checkpoints
            WHERE id = ?
            "#,
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        match row {
            Some(row) => Ok(Some(row.into_checkpoint()?)),
            None => Ok(None),
        }
    }

    /// List checkpoints for a project, ordered by created_at DESC (newest first)
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CheckpointInfo>> {
        let project_id_str = project_id.to_string();

        let rows: Vec<CheckpointInfoRow> = sqlx::query_as(
            r#"
            SELECT id, project_id, feature_id, description, size_bytes, created_at
            FROM checkpoints
            WHERE project_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(&project_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter()
            .map(|row| row.into_checkpoint_info())
            .collect()
    }

    /// Delete a checkpoint by ID
    pub async fn delete(&self, checkpoint_id: Uuid) -> Result<bool> {
        let id = checkpoint_id.to_string();

        let result = sqlx::query("DELETE FROM checkpoints WHERE id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all checkpoints for a project
    pub async fn delete_all_for_project(&self, project_id: Uuid) -> Result<u64> {
        let project_id_str = project_id.to_string();

        let result = sqlx::query("DELETE FROM checkpoints WHERE project_id = ?")
            .bind(&project_id_str)
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// Delete checkpoints older than the specified number of days
    pub async fn delete_older_than(&self, project_id: Uuid, days: i64) -> Result<u64> {
        let project_id_str = project_id.to_string();
        let cutoff = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM checkpoints
            WHERE project_id = ? AND created_at < ?
            "#,
        )
        .bind(&project_id_str)
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// Get phases for a project (for snapshot capture)
    pub async fn get_phases(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<(String, String, Option<String>, String, i32)>> {
        let project_id_str = project_id.to_string();

        let rows: Vec<(String, String, Option<String>, String, i32)> = sqlx::query_as(
            r#"
            SELECT id, name, description, status, order_index
            FROM phases
            WHERE project_id = ?
            ORDER BY order_index ASC
            "#,
        )
        .bind(&project_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(rows)
    }

    /// Get features for a project (for snapshot capture)
    pub async fn get_features(&self, project_id: Uuid) -> Result<Vec<FeatureRow>> {
        let project_id_str = project_id.to_string();

        let rows: Vec<FeatureRow> = sqlx::query_as(
            r#"
            SELECT id, title, description, status, phase_id, priority, acceptance_criteria, labels
            FROM features
            WHERE project_id = ?
            ORDER BY priority ASC, created_at ASC
            "#,
        )
        .bind(&project_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(rows)
    }

    /// Get recent messages for a project (for snapshot capture)
    pub async fn get_recent_messages(
        &self,
        project_id: Uuid,
        limit: i32,
    ) -> Result<Vec<(String, String, String, String, Option<String>)>> {
        let project_id_str = project_id.to_string();

        let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT m.id, m.conversation_id, m.role, m.content, m.model
            FROM messages m
            JOIN conversations c ON m.conversation_id = c.id
            WHERE c.project_id = ?
            ORDER BY m.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&project_id_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(rows)
    }

    /// Count checkpoints for a project
    pub async fn count_by_project(&self, project_id: Uuid) -> Result<i64> {
        let project_id_str = project_id.to_string();

        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM checkpoints WHERE project_id = ?
            "#,
        )
        .bind(&project_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(count)
    }
}

// ========== Trait Implementation ==========

#[async_trait]
impl CheckpointRepositoryTrait for CheckpointRepository {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        self.save(checkpoint).await
    }

    async fn get(&self, checkpoint_id: Uuid) -> Result<Option<Checkpoint>> {
        self.get(checkpoint_id).await
    }

    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CheckpointInfo>> {
        self.list_by_project(project_id).await
    }

    async fn delete(&self, checkpoint_id: Uuid) -> Result<bool> {
        self.delete(checkpoint_id).await
    }

    async fn delete_all_for_project(&self, project_id: Uuid) -> Result<u64> {
        self.delete_all_for_project(project_id).await
    }

    async fn delete_older_than(&self, project_id: Uuid, days: i64) -> Result<u64> {
        self.delete_older_than(project_id, days).await
    }

    async fn count_by_project(&self, project_id: Uuid) -> Result<i64> {
        self.count_by_project(project_id).await
    }

    async fn get_phases(&self, project_id: Uuid) -> Result<Vec<TraitPhaseRow>> {
        self.get_phases(project_id).await
    }

    async fn get_features(&self, project_id: Uuid) -> Result<Vec<TraitFeatureRow>> {
        self.get_features(project_id).await
    }

    async fn get_recent_messages(
        &self,
        project_id: Uuid,
        limit: i32,
    ) -> Result<Vec<TraitMessageRow>> {
        self.get_recent_messages(project_id, limit).await
    }
}

/// Database row for full checkpoint
#[derive(sqlx::FromRow)]
struct CheckpointRow {
    id: String,
    project_id: String,
    feature_id: Option<String>,
    description: String,
    snapshot_data: String,
    size_bytes: i64,
    signature: Vec<u8>,
    created_at: DateTime<Utc>,
}

impl CheckpointRow {
    fn into_checkpoint(self) -> Result<Checkpoint> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| Error::Parse(format!("Invalid checkpoint ID: {}", e)))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;
        let feature_id = self
            .feature_id
            .map(|f| Uuid::parse_str(&f))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid feature ID: {}", e)))?;
        let snapshot_data: serde_json::Value = serde_json::from_str(&self.snapshot_data)
            .map_err(|e| Error::Parse(format!("Invalid snapshot JSON: {}", e)))?;

        Ok(Checkpoint {
            id,
            project_id,
            feature_id,
            description: self.description,
            snapshot_data,
            size_bytes: self.size_bytes,
            signature: self.signature,
            created_at: self.created_at,
        })
    }
}

/// Database row for checkpoint info (without full snapshot data)
#[derive(sqlx::FromRow)]
struct CheckpointInfoRow {
    id: String,
    project_id: String,
    feature_id: Option<String>,
    description: String,
    size_bytes: i64,
    created_at: DateTime<Utc>,
}

impl CheckpointInfoRow {
    fn into_checkpoint_info(self) -> Result<CheckpointInfo> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| Error::Parse(format!("Invalid checkpoint ID: {}", e)))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;
        let feature_id = self
            .feature_id
            .map(|f| Uuid::parse_str(&f))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid feature ID: {}", e)))?;

        Ok(CheckpointInfo {
            id,
            project_id,
            feature_id,
            description: self.description,
            size_bytes: self.size_bytes,
            created_at: self.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    async fn create_test_db() -> SqlitePool {
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");
        db.pool().clone()
    }

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        let project_id = Uuid::new_v4();
        sqlx::query("INSERT INTO projects (id, name, framework) VALUES (?, ?, ?)")
            .bind(project_id.to_string())
            .bind("Test Project")
            .bind("rust")
            .execute(pool)
            .await
            .expect("Failed to insert test project");
        project_id
    }

    #[tokio::test]
    async fn test_save_and_get_checkpoint() {
        let pool = create_test_db().await;
        let project_id = create_test_project(&pool).await;
        let repo = CheckpointRepository::new(pool);

        let checkpoint = Checkpoint::new(
            project_id,
            None,
            "Test checkpoint".to_string(),
            serde_json::json!({"test": true}),
            vec![0u8; 64],
        );

        // Save
        repo.save(&checkpoint).await.expect("Failed to save");

        // Get
        let retrieved = repo
            .get(checkpoint.id)
            .await
            .expect("Failed to get")
            .expect("Checkpoint not found");

        assert_eq!(retrieved.id, checkpoint.id);
        assert_eq!(retrieved.description, checkpoint.description);
        assert_eq!(retrieved.size_bytes, checkpoint.size_bytes);
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let pool = create_test_db().await;
        let project_id = create_test_project(&pool).await;
        let repo = CheckpointRepository::new(pool);

        // Create multiple checkpoints
        for i in 0..3 {
            let checkpoint = Checkpoint::new(
                project_id,
                None,
                format!("Checkpoint {}", i),
                serde_json::json!({"index": i}),
                vec![0u8; 64],
            );
            repo.save(&checkpoint).await.expect("Failed to save");
        }

        let checkpoints = repo
            .list_by_project(project_id)
            .await
            .expect("Failed to list");
        assert_eq!(checkpoints.len(), 3);

        // Should be ordered by created_at DESC (newest first)
        assert!(checkpoints[0].description.contains("2"));
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let pool = create_test_db().await;
        let project_id = create_test_project(&pool).await;
        let repo = CheckpointRepository::new(pool);

        let checkpoint = Checkpoint::new(
            project_id,
            None,
            "To delete".to_string(),
            serde_json::json!({}),
            vec![0u8; 64],
        );
        repo.save(&checkpoint).await.expect("Failed to save");

        // Delete
        let deleted = repo.delete(checkpoint.id).await.expect("Failed to delete");
        assert!(deleted);

        // Should not exist
        let retrieved = repo.get(checkpoint.id).await.expect("Failed to get");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_count_checkpoints() {
        let pool = create_test_db().await;
        let project_id = create_test_project(&pool).await;
        let repo = CheckpointRepository::new(pool);

        // Initially 0
        let count = repo
            .count_by_project(project_id)
            .await
            .expect("Failed to count");
        assert_eq!(count, 0);

        // Add checkpoints
        for i in 0..5 {
            let checkpoint = Checkpoint::new(
                project_id,
                None,
                format!("Checkpoint {}", i),
                serde_json::json!({}),
                vec![0u8; 64],
            );
            repo.save(&checkpoint).await.expect("Failed to save");
        }

        let count = repo
            .count_by_project(project_id)
            .await
            .expect("Failed to count");
        assert_eq!(count, 5);
    }
}
