//! Checkpoint restoration service
//!
//! Handles restoring project state from a checkpoint, including database
//! rollback and file restoration.

use super::checkpoint::SnapshotData;
use super::manager::CheckpointManager;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Result of a checkpoint restoration
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// The checkpoint that was restored
    pub checkpoint_id: Uuid,

    /// Timestamp of the restored checkpoint
    pub checkpoint_timestamp: DateTime<Utc>,

    /// Description of the restored checkpoint
    pub checkpoint_description: String,

    /// Safety backup checkpoint created before restore
    pub safety_backup_id: Uuid,

    /// Number of phases restored
    pub phases_restored: usize,

    /// Number of features restored
    pub features_restored: usize,

    /// Number of messages restored
    pub messages_restored: usize,

    /// Number of generated code files restored
    pub files_restored: usize,
}

impl RestoreResult {
    /// Get a user-friendly summary message
    pub fn summary(&self) -> String {
        format!(
            "Project restored to state from {}. Restored {} phases, {} features, {} messages. Safety backup: {}",
            self.checkpoint_timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.phases_restored,
            self.features_restored,
            self.messages_restored,
            &self.safety_backup_id.to_string()[..8]
        )
    }
}

/// Errors specific to checkpoint restoration
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error("Checkpoint not found: {0}")]
    CheckpointNotFound(Uuid),

    #[error("Checkpoint signature verification failed")]
    SignatureVerificationFailed,

    #[error("Failed to deserialize snapshot data: {0}")]
    DeserializationFailed(String),

    #[error("Failed to create safety backup: {0}")]
    SafetyBackupFailed(String),

    #[error("Database restore failed: {0}")]
    DatabaseRestoreFailed(String),

    #[error("File restore failed: {0}")]
    FileRestoreFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

impl From<RestoreError> for Error {
    fn from(err: RestoreError) -> Self {
        Error::Other(err.to_string())
    }
}

/// Restore a checkpoint to project state
///
/// This function:
/// 1. Verifies the checkpoint signature
/// 2. Creates a safety backup before restore
/// 3. Restores database state within a transaction
/// 4. Restores any tracked files
///
/// Returns a RestoreResult on success containing details about what was restored.
pub async fn restore_checkpoint(
    pool: &SqlitePool,
    manager: &CheckpointManager,
    checkpoint_id: Uuid,
) -> Result<RestoreResult> {
    use std::time::Instant;

    let start = Instant::now();
    info!(checkpoint_id = %checkpoint_id, "Starting checkpoint restoration");

    // 1. Get the checkpoint
    let checkpoint = manager
        .get_checkpoint(checkpoint_id)
        .await?
        .ok_or(RestoreError::CheckpointNotFound(checkpoint_id))?;

    // 2. Verify signature
    debug!("Verifying checkpoint signature");
    manager
        .verify_checkpoint(&checkpoint)
        .map_err(|_| RestoreError::SignatureVerificationFailed)?;
    info!("Checkpoint signature verified");

    // 3. Deserialize snapshot data
    let snapshot: SnapshotData = serde_json::from_value(checkpoint.snapshot_data.clone())
        .map_err(|e| RestoreError::DeserializationFailed(e.to_string()))?;

    // 4. Create safety backup before restore
    info!("Creating safety backup before restore");
    let safety_backup = manager
        .create_checkpoint(
            checkpoint.project_id,
            checkpoint.feature_id,
            format!(
                "Auto-backup before restore to checkpoint {}",
                &checkpoint_id.to_string()[..8]
            ),
        )
        .await
        .map_err(|e| RestoreError::SafetyBackupFailed(e.to_string()))?;
    info!(safety_backup_id = %safety_backup.id, "Safety backup created");

    // 5. Restore database state within a transaction
    let (phases_restored, features_restored, messages_restored) =
        restore_database_state(pool, checkpoint.project_id, &snapshot).await?;

    // 6. Restore files (if any were tracked)
    let files_restored = restore_files(&snapshot).await?;

    let elapsed = start.elapsed();
    let result = RestoreResult {
        checkpoint_id,
        checkpoint_timestamp: checkpoint.created_at,
        checkpoint_description: checkpoint.description.clone(),
        safety_backup_id: safety_backup.id,
        phases_restored,
        features_restored,
        messages_restored,
        files_restored,
    };

    info!(
        checkpoint_id = %checkpoint_id,
        phases = phases_restored,
        features = features_restored,
        messages = messages_restored,
        files = files_restored,
        elapsed_ms = elapsed.as_millis(),
        "Checkpoint restoration complete"
    );

    // Warn if restore took longer than 5 seconds (performance requirement)
    if elapsed.as_secs() > 5 {
        warn!(
            elapsed_secs = elapsed.as_secs(),
            "Checkpoint restoration exceeded 5 second target"
        );
    }

    Ok(result)
}

/// Restore database state from snapshot within a transaction
async fn restore_database_state(
    pool: &SqlitePool,
    project_id: Uuid,
    snapshot: &SnapshotData,
) -> Result<(usize, usize, usize)> {
    let project_id_str = project_id.to_string();

    // Start a transaction for atomicity
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| RestoreError::TransactionFailed(e.to_string()))?;

    // Clear existing data for this project (in dependency order)
    debug!("Clearing existing project data");

    // Delete messages first (they reference conversations)
    sqlx::query(
        r#"
        DELETE FROM messages
        WHERE conversation_id IN (SELECT id FROM conversations WHERE project_id = ?)
        "#,
    )
    .bind(&project_id_str)
    .execute(&mut *tx)
    .await
    .map_err(|e| RestoreError::DatabaseRestoreFailed(format!("Failed to clear messages: {}", e)))?;

    // Delete features (they reference phases)
    sqlx::query("DELETE FROM features WHERE project_id = ?")
        .bind(&project_id_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            RestoreError::DatabaseRestoreFailed(format!("Failed to clear features: {}", e))
        })?;

    // Delete phases
    sqlx::query("DELETE FROM phases WHERE project_id = ?")
        .bind(&project_id_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            RestoreError::DatabaseRestoreFailed(format!("Failed to clear phases: {}", e))
        })?;

    // Restore phases
    debug!("Restoring {} phases", snapshot.phases.len());
    for phase in &snapshot.phases {
        sqlx::query(
            r#"
            INSERT INTO phases (id, project_id, name, description, status, order_index)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&phase.id)
        .bind(&project_id_str)
        .bind(&phase.name)
        .bind(&phase.description)
        .bind(&phase.status)
        .bind(phase.order_index)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            RestoreError::DatabaseRestoreFailed(format!("Failed to insert phase: {}", e))
        })?;
    }

    // Restore features
    debug!("Restoring {} features", snapshot.features.len());
    for feature in &snapshot.features {
        sqlx::query(
            r#"
            INSERT INTO features (id, project_id, title, description, status, phase_id, priority, acceptance_criteria, labels)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&feature.id)
        .bind(&project_id_str)
        .bind(&feature.title)
        .bind(&feature.description)
        .bind(&feature.status)
        .bind(&feature.phase_id)
        .bind(feature.priority)
        .bind(&feature.acceptance_criteria)
        .bind(&feature.labels)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            RestoreError::DatabaseRestoreFailed(format!("Failed to insert feature: {}", e))
        })?;
    }

    // Restore messages
    // Note: We need to ensure conversations exist for these messages
    debug!("Restoring {} messages", snapshot.chat_messages.len());
    for message in &snapshot.chat_messages {
        // Ensure conversation exists (upsert)
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO conversations (id, project_id, title, created_at, updated_at)
            VALUES (?, ?, 'Restored Conversation', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&message.conversation_id)
        .bind(&project_id_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            RestoreError::DatabaseRestoreFailed(format!("Failed to ensure conversation: {}", e))
        })?;

        // Insert message
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO messages (id, conversation_id, role, content, model, created_at)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(&message.model)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            RestoreError::DatabaseRestoreFailed(format!("Failed to insert message: {}", e))
        })?;
    }

    // Commit the transaction
    tx.commit()
        .await
        .map_err(|e| RestoreError::TransactionFailed(format!("Failed to commit: {}", e)))?;

    Ok((
        snapshot.phases.len(),
        snapshot.features.len(),
        snapshot.chat_messages.len(),
    ))
}

/// Restore files from snapshot
///
/// Currently, generated code files are not tracked in snapshots.
/// This is a placeholder for future file restoration functionality.
async fn restore_files(snapshot: &SnapshotData) -> Result<usize> {
    if snapshot.generated_code.is_empty() {
        debug!("No generated code files to restore");
        return Ok(0);
    }

    let mut restored_count = 0;

    for code_file in &snapshot.generated_code {
        debug!(path = %code_file.path, "Restoring generated code file");

        // Ensure parent directory exists
        let path = std::path::Path::new(&code_file.path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    RestoreError::FileRestoreFailed(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        // Write file content
        std::fs::write(path, &code_file.content).map_err(|e| {
            RestoreError::FileRestoreFailed(format!(
                "Failed to write file {}: {}",
                code_file.path, e
            ))
        })?;

        restored_count += 1;
    }

    if restored_count > 0 {
        info!(count = restored_count, "Restored generated code files");
    }

    Ok(restored_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recovery::signing::CheckpointSigner;
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

    async fn create_test_phase(pool: &SqlitePool, project_id: Uuid, name: &str) -> String {
        let phase_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO phases (id, project_id, name, status, order_index) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&phase_id)
        .bind(project_id.to_string())
        .bind(name)
        .bind("pending")
        .bind(0)
        .execute(pool)
        .await
        .expect("Failed to insert test phase");
        phase_id
    }

    #[tokio::test]
    async fn test_restore_result_summary() {
        let result = RestoreResult {
            checkpoint_id: Uuid::new_v4(),
            checkpoint_timestamp: Utc::now(),
            checkpoint_description: "Test checkpoint".to_string(),
            safety_backup_id: Uuid::new_v4(),
            phases_restored: 3,
            features_restored: 5,
            messages_restored: 10,
            files_restored: 2,
        };

        let summary = result.summary();
        assert!(summary.contains("3 phases"));
        assert!(summary.contains("5 features"));
        assert!(summary.contains("10 messages"));
    }

    #[tokio::test]
    async fn test_restore_checkpoint_not_found() {
        let pool = create_test_db().await;
        let signer = CheckpointSigner::generate();
        let manager = CheckpointManager::new(pool.clone(), signer);

        let result = restore_checkpoint(&pool, &manager, Uuid::new_v4()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Checkpoint not found"));
    }

    #[tokio::test]
    async fn test_full_restore_cycle() {
        let pool = create_test_db().await;
        let project_id = create_test_project(&pool).await;
        let _phase_id = create_test_phase(&pool, project_id, "Initial Phase").await;

        let signer = CheckpointSigner::generate();
        let manager = CheckpointManager::new(pool.clone(), signer);

        // Create a checkpoint with the current state
        let checkpoint = manager
            .create_checkpoint(project_id, None, "Test checkpoint".to_string())
            .await
            .expect("Failed to create checkpoint");

        // Modify the database (add another phase)
        create_test_phase(&pool, project_id, "New Phase").await;

        // Count phases before restore
        let (count_before,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM phases WHERE project_id = ?")
                .bind(project_id.to_string())
                .fetch_one(&pool)
                .await
                .expect("Failed to count phases");
        assert_eq!(count_before, 2);

        // Restore the checkpoint
        let result = restore_checkpoint(&pool, &manager, checkpoint.id)
            .await
            .expect("Failed to restore checkpoint");

        // Verify restore result
        assert_eq!(result.checkpoint_id, checkpoint.id);
        assert_eq!(result.phases_restored, 1);

        // Count phases after restore
        let (count_after,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM phases WHERE project_id = ?")
                .bind(project_id.to_string())
                .fetch_one(&pool)
                .await
                .expect("Failed to count phases");
        assert_eq!(count_after, 1);

        // Verify safety backup was created
        let checkpoints = manager
            .list_checkpoints(project_id)
            .await
            .expect("Failed to list checkpoints");
        assert!(checkpoints.len() >= 2); // Original + safety backup
    }
}
