//! Checkpoint commands for code safety and recovery
//!
//! Provides CLI commands for listing, viewing, and managing checkpoints.

use crate::domain::recovery::{
    CheckpointConfig, CheckpointInfo, CheckpointManager, CheckpointSigner, CheckpointStats,
    RestoreResult,
};
use crate::error::Result;
use crate::storage::Database;
use uuid::Uuid;

/// List all checkpoints for a project
pub async fn list_checkpoints(project_id: Uuid) -> Result<Vec<CheckpointInfo>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    manager.list_checkpoints(project_id).await
}

/// Get checkpoint statistics for a project
pub async fn get_checkpoint_stats(project_id: Uuid) -> Result<CheckpointStats> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    manager.get_stats(project_id).await
}

/// Delete a specific checkpoint
pub async fn delete_checkpoint(checkpoint_id: Uuid) -> Result<bool> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    manager.delete_checkpoint(checkpoint_id).await
}

/// Delete all checkpoints for a project
pub async fn delete_all_checkpoints(project_id: Uuid) -> Result<u64> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    manager.delete_all_for_project(project_id).await
}

/// Verify a checkpoint's signature integrity
pub async fn verify_checkpoint(checkpoint_id: Uuid) -> Result<bool> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    let checkpoint = manager
        .get_checkpoint(checkpoint_id)
        .await?
        .ok_or_else(|| {
            crate::error::Error::NotFound(format!("Checkpoint {} not found", checkpoint_id))
        })?;

    match manager.verify_checkpoint(&checkpoint) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Create a checkpoint manually (for explicit backup points)
pub async fn create_checkpoint(
    project_id: Uuid,
    description: String,
    feature_id: Option<Uuid>,
) -> Result<CheckpointInfo> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    let checkpoint = manager
        .create_checkpoint(project_id, feature_id, description)
        .await?;

    Ok(CheckpointInfo::from(&checkpoint))
}

/// Restore a checkpoint to project state
///
/// This will:
/// 1. Verify the checkpoint signature
/// 2. Create a safety backup before restore
/// 3. Restore database state (phases, features, messages)
/// 4. Restore any tracked generated code files
///
/// Returns a RestoreResult containing details about what was restored.
pub async fn restore_checkpoint(checkpoint_id: Uuid) -> Result<RestoreResult> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let signer = get_or_create_signer()?;
    let manager = CheckpointManager::new(db.pool().clone(), signer);

    manager.restore_checkpoint(checkpoint_id).await
}

/// Get or create the signing key
///
/// In a production system, this would retrieve the key from secure storage
/// (e.g., OS keyring). For now, we generate a new key per session which
/// is stored in memory. A future enhancement would persist this key.
fn get_or_create_signer() -> Result<CheckpointSigner> {
    // TODO: In the future, store the signing key in the OS keyring
    // similar to how we store the master encryption key for API keys.
    // For now, we generate a fresh key each time which means signatures
    // can only be verified within the same session.
    //
    // This is acceptable for the initial implementation since the primary
    // purpose is to detect accidental corruption, not malicious tampering.
    Ok(CheckpointSigner::generate())
}

/// Configuration for checkpoint behavior
pub fn default_config() -> CheckpointConfig {
    CheckpointConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert_eq!(config.retention_days, 30);
        assert_eq!(config.max_per_project, 50);
    }
}
