//! Recovery domain module
//!
//! Provides automatic checkpointing, recovery, and edit detection for code safety.
//!
//! # Architecture
//!
//! - **Entities**: `Checkpoint`, `SnapshotData`, `CheckpointInfo`, `TrackedFile`
//! - **Repository**: `CheckpointRepository` and `TrackedFileRepository` for database operations
//! - **Manager**: `CheckpointManager` for orchestrating checkpoint operations
//! - **Signing**: Ed25519 signing for checkpoint integrity verification
//! - **Edit Detection**: `EditDetectionService` for tracking user modifications
//!
//! # Features
//!
//! - Automatic checkpoint creation before major changes (code generation, document updates)
//! - Ed25519 signature verification for data integrity
//! - Configurable retention policy (days and max count)
//! - Snapshot of full project state (phases, features, messages, generated code)
//! - User edit detection for generated code files
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::recovery::{CheckpointManager, CheckpointSigner, EditDetectionService};
//! use sqlx::SqlitePool;
//!
//! // Create signing key (normally stored securely)
//! let signer = CheckpointSigner::generate();
//!
//! // Create manager
//! let manager = CheckpointManager::new(pool.clone(), signer);
//!
//! // Create checkpoint before code generation
//! let checkpoint = manager
//!     .create_before_generation(project_id, Some(feature_id), "User Auth")
//!     .await?;
//!
//! // Track generated files for edit detection
//! let edit_service = EditDetectionService::new(pool);
//! edit_service.track_generated_file(project_id, Some(feature_id), "src/auth.rs", &content).await?;
//!
//! // Later, check for user edits
//! let summary = edit_service.check_all_files(project_id).await?;
//! if summary.has_changes() {
//!     println!("User has modified {} files", summary.modified_files.len());
//! }
//! ```

pub mod checkpoint;
pub mod edit_detection;
pub mod manager;
pub mod repository;
pub mod restore;
pub mod signing;

// Re-export main types
pub use checkpoint::{
    Checkpoint, CheckpointInfo, FeatureSnapshot, GeneratedCodeSnapshot, MessageSnapshot,
    PhaseSnapshot, SnapshotData,
};
pub use edit_detection::{
    EditCheckResult, EditDetectionService, EditDetectionSummary, TrackedFile,
    TrackedFileRepository, compute_content_hash as compute_file_hash,
};
pub use manager::{
    CheckpointConfig, CheckpointManager, CheckpointStats, DEFAULT_MAX_PER_PROJECT,
    DEFAULT_RETENTION_DAYS, compute_content_hash,
};
pub use repository::CheckpointRepository;
pub use restore::{RestoreError, RestoreResult, restore_checkpoint};
pub use signing::{
    CheckpointSigner, CheckpointVerifier, PRIVATE_KEY_SIZE, PUBLIC_KEY_SIZE, SIGNATURE_SIZE,
    SigningError,
};
