//! Repository trait for checkpoint persistence
//!
//! This module defines the trait for checkpoint storage operations.
//! The trait abstracts over different storage backends (SQLite, etc.).

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;

use super::checkpoint::{Checkpoint, CheckpointInfo};

/// Type alias for phase database row
/// (id, name, description, status, order_index)
pub type PhaseRow = (String, String, Option<String>, String, i32);

/// Type alias for feature database row
/// (id, title, description, status, phase_id, priority, acceptance_criteria, labels)
pub type FeatureRow = (
    String,         // id
    String,         // title
    Option<String>, // description
    String,         // status
    Option<String>, // phase_id
    i32,            // priority
    Option<String>, // acceptance_criteria
    Option<String>, // labels
);

/// Type alias for message database row
/// (id, conversation_id, role, content, model)
pub type MessageRow = (String, String, String, String, Option<String>);

/// Repository trait for checkpoint persistence
///
/// Provides CRUD operations for checkpoints and snapshot data retrieval.
#[async_trait]
pub trait CheckpointRepositoryTrait: Send + Sync {
    // ========== Checkpoint CRUD ==========

    /// Save a checkpoint to the database
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()>;

    /// Get a checkpoint by ID
    async fn get(&self, checkpoint_id: Uuid) -> Result<Option<Checkpoint>>;

    /// List checkpoints for a project, ordered by created_at DESC (newest first)
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CheckpointInfo>>;

    /// Delete a checkpoint by ID
    async fn delete(&self, checkpoint_id: Uuid) -> Result<bool>;

    /// Delete all checkpoints for a project
    async fn delete_all_for_project(&self, project_id: Uuid) -> Result<u64>;

    /// Delete checkpoints older than the specified number of days
    async fn delete_older_than(&self, project_id: Uuid, days: i64) -> Result<u64>;

    /// Count checkpoints for a project
    async fn count_by_project(&self, project_id: Uuid) -> Result<i64>;

    // ========== Snapshot Data Retrieval ==========

    /// Get phases for a project (for snapshot capture)
    async fn get_phases(&self, project_id: Uuid) -> Result<Vec<PhaseRow>>;

    /// Get features for a project (for snapshot capture)
    async fn get_features(&self, project_id: Uuid) -> Result<Vec<FeatureRow>>;

    /// Get recent messages for a project (for snapshot capture)
    async fn get_recent_messages(&self, project_id: Uuid, limit: i32) -> Result<Vec<MessageRow>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify trait is object-safe
    fn _assert_object_safe(_: &dyn CheckpointRepositoryTrait) {}
}
