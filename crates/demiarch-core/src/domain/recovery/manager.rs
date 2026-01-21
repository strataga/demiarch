//! Checkpoint manager service
//!
//! Orchestrates automatic checkpoint creation before major changes
//! and provides methods for listing, verifying, and managing checkpoints.

use super::checkpoint::{
    Checkpoint, CheckpointInfo, FeatureSnapshot, GeneratedCodeSnapshot, MessageSnapshot,
    PhaseSnapshot, SnapshotData,
};
use super::repository::CheckpointRepository;
use super::restore::{self, RestoreResult};
use super::signing::{CheckpointSigner, CheckpointVerifier, SigningError};
use crate::error::Result;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tracing::{debug, info};
use uuid::Uuid;

/// Default retention days for checkpoints
pub const DEFAULT_RETENTION_DAYS: i64 = 30;

/// Default maximum checkpoints per project
pub const DEFAULT_MAX_PER_PROJECT: usize = 50;

/// Configuration for the checkpoint manager
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Number of days to retain checkpoints
    pub retention_days: i64,

    /// Maximum number of checkpoints per project
    pub max_per_project: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            retention_days: DEFAULT_RETENTION_DAYS,
            max_per_project: DEFAULT_MAX_PER_PROJECT,
        }
    }
}

/// Checkpoint manager service
///
/// Handles automatic checkpoint creation, verification, and cleanup.
pub struct CheckpointManager {
    repository: CheckpointRepository,
    signer: CheckpointSigner,
    config: CheckpointConfig,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(pool: SqlitePool, signer: CheckpointSigner) -> Self {
        Self {
            repository: CheckpointRepository::new(pool),
            signer,
            config: CheckpointConfig::default(),
        }
    }

    /// Create a checkpoint manager with custom configuration
    pub fn with_config(pool: SqlitePool, signer: CheckpointSigner, config: CheckpointConfig) -> Self {
        Self {
            repository: CheckpointRepository::new(pool),
            signer,
            config,
        }
    }

    /// Get a reference to the underlying repository
    pub fn repository(&self) -> &CheckpointRepository {
        &self.repository
    }

    /// Create a checkpoint before code generation
    ///
    /// This captures the current project state including phases, features,
    /// chat messages, and any existing generated code files.
    pub async fn create_before_generation(
        &self,
        project_id: Uuid,
        feature_id: Option<Uuid>,
        feature_name: &str,
    ) -> Result<Checkpoint> {
        let description = format!("Before generating {}", feature_name);
        info!(
            project_id = %project_id,
            feature_name = %feature_name,
            "Creating checkpoint before code generation"
        );

        self.create_checkpoint(project_id, feature_id, description)
            .await
    }

    /// Create a checkpoint before document update
    pub async fn create_before_document_update(
        &self,
        project_id: Uuid,
        document_type: &str,
    ) -> Result<Checkpoint> {
        let description = format!("Before updating {} document", document_type);
        info!(
            project_id = %project_id,
            document_type = %document_type,
            "Creating checkpoint before document update"
        );

        self.create_checkpoint(project_id, None, description).await
    }

    /// Create a checkpoint with custom description
    pub async fn create_checkpoint(
        &self,
        project_id: Uuid,
        feature_id: Option<Uuid>,
        description: String,
    ) -> Result<Checkpoint> {
        // Capture current project state
        let snapshot_data = self.capture_project_state(project_id).await?;
        let snapshot_json = serde_json::to_value(&snapshot_data)
            .map_err(|e| crate::error::Error::Other(format!("Failed to serialize snapshot: {}", e)))?;

        // Sign the snapshot data
        let snapshot_bytes = serde_json::to_vec(&snapshot_json)
            .map_err(|e| crate::error::Error::Other(format!("Failed to serialize for signing: {}", e)))?;
        let signature = self.signer.sign(&snapshot_bytes);

        // Create the checkpoint
        let checkpoint = Checkpoint::new(
            project_id,
            feature_id,
            description,
            snapshot_json,
            signature,
        );

        debug!(
            checkpoint_id = %checkpoint.id,
            size_bytes = checkpoint.size_bytes,
            "Checkpoint created"
        );

        // Save to database
        self.repository.save(&checkpoint).await?;

        // Enforce retention policy
        self.enforce_retention_policy(project_id).await?;

        Ok(checkpoint)
    }

    /// List all checkpoints for a project
    pub async fn list_checkpoints(&self, project_id: Uuid) -> Result<Vec<CheckpointInfo>> {
        self.repository.list_by_project(project_id).await
    }

    /// Get a checkpoint by ID
    pub async fn get_checkpoint(&self, checkpoint_id: Uuid) -> Result<Option<Checkpoint>> {
        self.repository.get(checkpoint_id).await
    }

    /// Verify a checkpoint's signature
    pub fn verify_checkpoint(&self, checkpoint: &Checkpoint) -> std::result::Result<(), SigningError> {
        let snapshot_bytes = serde_json::to_vec(&checkpoint.snapshot_data)
            .map_err(|e| SigningError::SigningFailed(format!("Failed to serialize: {}", e)))?;

        self.signer.verify(&snapshot_bytes, &checkpoint.signature)
    }

    /// Verify a checkpoint using only the public key
    pub fn verify_with_public_key(
        checkpoint: &Checkpoint,
        public_key: &[u8],
    ) -> std::result::Result<(), SigningError> {
        let verifier = CheckpointVerifier::from_bytes(public_key)?;
        let snapshot_bytes = serde_json::to_vec(&checkpoint.snapshot_data)
            .map_err(|e| SigningError::SigningFailed(format!("Failed to serialize: {}", e)))?;

        verifier.verify(&snapshot_bytes, &checkpoint.signature)
    }

    /// Delete a checkpoint
    pub async fn delete_checkpoint(&self, checkpoint_id: Uuid) -> Result<bool> {
        self.repository.delete(checkpoint_id).await
    }

    /// Delete all checkpoints for a project
    pub async fn delete_all_for_project(&self, project_id: Uuid) -> Result<u64> {
        self.repository.delete_all_for_project(project_id).await
    }

    /// Enforce retention policy by removing old checkpoints
    async fn enforce_retention_policy(&self, project_id: Uuid) -> Result<()> {
        // Delete checkpoints older than retention period
        let deleted_old = self
            .repository
            .delete_older_than(project_id, self.config.retention_days)
            .await?;

        if deleted_old > 0 {
            debug!(
                project_id = %project_id,
                deleted = deleted_old,
                "Deleted old checkpoints due to retention policy"
            );
        }

        // Enforce max per project limit
        let checkpoints = self.repository.list_by_project(project_id).await?;
        if checkpoints.len() > self.config.max_per_project {
            let to_delete = checkpoints.len() - self.config.max_per_project;
            // Delete oldest checkpoints (list is ordered by created_at DESC)
            for checkpoint in checkpoints.iter().rev().take(to_delete) {
                self.repository.delete(checkpoint.id).await?;
                debug!(
                    checkpoint_id = %checkpoint.id,
                    "Deleted checkpoint due to max limit"
                );
            }
        }

        Ok(())
    }

    /// Capture the current project state
    async fn capture_project_state(&self, project_id: Uuid) -> Result<SnapshotData> {
        // Query phases
        let phases = self.repository.get_phases(project_id).await?;
        let phase_snapshots: Vec<PhaseSnapshot> = phases
            .into_iter()
            .map(|p| PhaseSnapshot {
                id: p.0,
                name: p.1,
                description: p.2,
                status: p.3,
                order_index: p.4,
            })
            .collect();

        // Query features
        let features = self.repository.get_features(project_id).await?;
        let feature_snapshots: Vec<FeatureSnapshot> = features
            .into_iter()
            .map(|f| FeatureSnapshot {
                id: f.0,
                title: f.1,
                description: f.2,
                status: f.3,
                phase_id: f.4,
                priority: f.5,
                acceptance_criteria: f.6,
                labels: f.7,
            })
            .collect();

        // Query recent chat messages (limit to prevent huge snapshots)
        let messages = self.repository.get_recent_messages(project_id, 100).await?;
        let message_snapshots: Vec<MessageSnapshot> = messages
            .into_iter()
            .map(|m| MessageSnapshot {
                id: m.0,
                conversation_id: m.1,
                role: m.2,
                content: m.3,
                model: m.4,
            })
            .collect();

        // Note: generated_code is not stored in DB; it would need to be captured
        // from the filesystem. For now we leave it empty; this can be enhanced
        // in a future iteration to scan project files.
        let generated_code: Vec<GeneratedCodeSnapshot> = Vec::new();

        Ok(SnapshotData {
            phases: phase_snapshots,
            features: feature_snapshots,
            chat_messages: message_snapshots,
            generated_code,
        })
    }

    /// Restore a checkpoint to project state
    ///
    /// This will:
    /// 1. Verify the checkpoint signature
    /// 2. Create a safety backup before restore
    /// 3. Restore database state within a transaction
    /// 4. Restore any tracked files
    ///
    /// Returns a RestoreResult on success containing details about what was restored.
    pub async fn restore_checkpoint(&self, checkpoint_id: Uuid) -> Result<RestoreResult> {
        restore::restore_checkpoint(self.repository.pool(), self, checkpoint_id).await
    }

    /// Get statistics about checkpoints for a project
    pub async fn get_stats(&self, project_id: Uuid) -> Result<CheckpointStats> {
        let checkpoints = self.repository.list_by_project(project_id).await?;
        let total_count = checkpoints.len();
        let total_size: i64 = checkpoints.iter().map(|c| c.size_bytes).sum();

        let oldest = checkpoints.last().map(|c| c.created_at);
        let newest = checkpoints.first().map(|c| c.created_at);

        Ok(CheckpointStats {
            total_count,
            total_size,
            oldest_checkpoint: oldest,
            newest_checkpoint: newest,
        })
    }
}

/// Statistics about checkpoints for a project
#[derive(Debug, Clone)]
pub struct CheckpointStats {
    /// Total number of checkpoints
    pub total_count: usize,

    /// Total size in bytes
    pub total_size: i64,

    /// Oldest checkpoint timestamp
    pub oldest_checkpoint: Option<chrono::DateTime<chrono::Utc>>,

    /// Newest checkpoint timestamp
    pub newest_checkpoint: Option<chrono::DateTime<chrono::Utc>>,
}

impl CheckpointStats {
    /// Get display-friendly total size
    pub fn display_total_size(&self) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if self.total_size >= GB {
            format!("{:.2} GB", self.total_size as f64 / GB as f64)
        } else if self.total_size >= MB {
            format!("{:.2} MB", self.total_size as f64 / MB as f64)
        } else if self.total_size >= KB {
            format!("{:.2} KB", self.total_size as f64 / KB as f64)
        } else {
            format!("{} B", self.total_size)
        }
    }
}

/// Compute SHA-256 hash of content
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert_eq!(config.retention_days, DEFAULT_RETENTION_DAYS);
        assert_eq!(config.max_per_project, DEFAULT_MAX_PER_PROJECT);
    }

    #[test]
    fn test_compute_content_hash() {
        let content = "Hello, world!";
        let hash = compute_content_hash(content);

        // SHA-256 of "Hello, world!" is well-known
        assert_eq!(
            hash,
            "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3"
        );
    }

    #[test]
    fn test_checkpoint_stats_display_size() {
        let stats = CheckpointStats {
            total_count: 5,
            total_size: 1_500_000,
            oldest_checkpoint: None,
            newest_checkpoint: None,
        };

        assert_eq!(stats.display_total_size(), "1.43 MB");
    }
}
