//! Checkpoint entity for code safety and recovery
//!
//! Represents a snapshot of project state before destructive operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Checkpoint entity representing a project state snapshot
///
/// Created automatically before major changes (code generation, document updates)
/// to enable rollback if something goes wrong.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier for this checkpoint
    pub id: Uuid,

    /// The project this checkpoint belongs to
    pub project_id: Uuid,

    /// Optional feature ID if checkpoint was created for a specific feature
    pub feature_id: Option<Uuid>,

    /// Human-readable description (e.g., "Before generating User Auth")
    pub description: String,

    /// JSON blob containing the full project state snapshot
    pub snapshot_data: serde_json::Value,

    /// Size of the checkpoint data in bytes
    pub size_bytes: i64,

    /// Ed25519 signature for integrity verification
    pub signature: Vec<u8>,

    /// When this checkpoint was created
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    /// Create a new checkpoint with the given data
    ///
    /// Note: The signature should be computed by the CheckpointManager
    /// using the ed25519 signing service.
    pub fn new(
        project_id: Uuid,
        feature_id: Option<Uuid>,
        description: String,
        snapshot_data: serde_json::Value,
        signature: Vec<u8>,
    ) -> Self {
        let snapshot_str = snapshot_data.to_string();
        let size_bytes = snapshot_str.len() as i64;

        Self {
            id: Uuid::new_v4(),
            project_id,
            feature_id,
            description,
            snapshot_data,
            size_bytes,
            signature,
            created_at: Utc::now(),
        }
    }

    /// Get a display-friendly size (e.g., "1.2 KB", "3.5 MB")
    pub fn display_size(&self) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if self.size_bytes >= GB {
            format!("{:.2} GB", self.size_bytes as f64 / GB as f64)
        } else if self.size_bytes >= MB {
            format!("{:.2} MB", self.size_bytes as f64 / MB as f64)
        } else if self.size_bytes >= KB {
            format!("{:.2} KB", self.size_bytes as f64 / KB as f64)
        } else {
            format!("{} B", self.size_bytes)
        }
    }
}

/// Snapshot data structure containing the full project state
///
/// This is what gets serialized into `snapshot_data` JSON field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    /// All phases in the project
    pub phases: Vec<PhaseSnapshot>,

    /// All features in the project
    pub features: Vec<FeatureSnapshot>,

    /// Recent chat messages (limited to prevent huge snapshots)
    pub chat_messages: Vec<MessageSnapshot>,

    /// Generated code files that would be affected
    pub generated_code: Vec<GeneratedCodeSnapshot>,
}

impl SnapshotData {
    /// Create an empty snapshot
    pub fn empty() -> Self {
        Self {
            phases: Vec::new(),
            features: Vec::new(),
            chat_messages: Vec::new(),
            generated_code: Vec::new(),
        }
    }
}

/// Snapshot of a phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSnapshot {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub order_index: i32,
}

/// Snapshot of a feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSnapshot {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub phase_id: Option<String>,
    pub priority: i32,
    pub acceptance_criteria: Option<String>,
    pub labels: Option<String>,
}

/// Snapshot of a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSnapshot {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
}

/// Snapshot of generated code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCodeSnapshot {
    /// Relative file path
    pub path: String,

    /// File content
    pub content: String,

    /// SHA-256 hash of content for quick comparison
    pub content_hash: String,
}

/// Metadata for listing checkpoints (without full snapshot data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    /// Unique identifier
    pub id: Uuid,

    /// Project ID
    pub project_id: Uuid,

    /// Optional feature ID
    pub feature_id: Option<Uuid>,

    /// Human-readable description
    pub description: String,

    /// Size in bytes
    pub size_bytes: i64,

    /// When created
    pub created_at: DateTime<Utc>,
}

impl CheckpointInfo {
    /// Get a display-friendly size
    pub fn display_size(&self) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if self.size_bytes >= GB {
            format!("{:.2} GB", self.size_bytes as f64 / GB as f64)
        } else if self.size_bytes >= MB {
            format!("{:.2} MB", self.size_bytes as f64 / MB as f64)
        } else if self.size_bytes >= KB {
            format!("{:.2} KB", self.size_bytes as f64 / KB as f64)
        } else {
            format!("{} B", self.size_bytes)
        }
    }
}

impl From<&Checkpoint> for CheckpointInfo {
    fn from(checkpoint: &Checkpoint) -> Self {
        Self {
            id: checkpoint.id,
            project_id: checkpoint.project_id,
            feature_id: checkpoint.feature_id,
            description: checkpoint.description.clone(),
            size_bytes: checkpoint.size_bytes,
            created_at: checkpoint.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let snapshot = serde_json::json!({
            "phases": [],
            "features": [],
            "chat_messages": [],
            "generated_code": []
        });

        let checkpoint = Checkpoint::new(
            Uuid::new_v4(),
            None,
            "Test checkpoint".to_string(),
            snapshot,
            vec![0u8; 64], // Dummy signature
        );

        assert!(!checkpoint.description.is_empty());
        assert!(checkpoint.size_bytes > 0);
    }

    #[test]
    fn test_display_size() {
        let mut checkpoint = Checkpoint::new(
            Uuid::new_v4(),
            None,
            "Test".to_string(),
            serde_json::json!({}),
            vec![],
        );

        checkpoint.size_bytes = 500;
        assert_eq!(checkpoint.display_size(), "500 B");

        checkpoint.size_bytes = 1536;
        assert_eq!(checkpoint.display_size(), "1.50 KB");

        checkpoint.size_bytes = 1_500_000;
        assert_eq!(checkpoint.display_size(), "1.43 MB");

        checkpoint.size_bytes = 1_500_000_000;
        assert_eq!(checkpoint.display_size(), "1.40 GB");
    }

    #[test]
    fn test_snapshot_data_empty() {
        let snapshot = SnapshotData::empty();
        assert!(snapshot.phases.is_empty());
        assert!(snapshot.features.is_empty());
        assert!(snapshot.chat_messages.is_empty());
        assert!(snapshot.generated_code.is_empty());
    }

    #[test]
    fn test_checkpoint_info_from() {
        let checkpoint = Checkpoint::new(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            "Before generating".to_string(),
            serde_json::json!({"test": true}),
            vec![1, 2, 3],
        );

        let info = CheckpointInfo::from(&checkpoint);
        assert_eq!(info.id, checkpoint.id);
        assert_eq!(info.project_id, checkpoint.project_id);
        assert_eq!(info.feature_id, checkpoint.feature_id);
        assert_eq!(info.description, checkpoint.description);
        assert_eq!(info.size_bytes, checkpoint.size_bytes);
    }
}
