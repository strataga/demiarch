//! Recovery domain module
//!
//! Provides automatic checkpointing and recovery functionality for code safety.
//!
//! # Architecture
//!
//! - **Entities**: `Checkpoint`, `SnapshotData`, `CheckpointInfo`
//! - **Repository**: `CheckpointRepository` for database operations
//! - **Manager**: `CheckpointManager` for orchestrating checkpoint operations
//! - **Signing**: Ed25519 signing for checkpoint integrity verification
//!
//! # Features
//!
//! - Automatic checkpoint creation before major changes (code generation, document updates)
//! - Ed25519 signature verification for data integrity
//! - Configurable retention policy (days and max count)
//! - Snapshot of full project state (phases, features, messages, generated code)
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::recovery::{CheckpointManager, CheckpointSigner};
//! use sqlx::SqlitePool;
//!
//! // Create signing key (normally stored securely)
//! let signer = CheckpointSigner::generate();
//!
//! // Create manager
//! let manager = CheckpointManager::new(pool, signer);
//!
//! // Create checkpoint before code generation
//! let checkpoint = manager
//!     .create_before_generation(project_id, Some(feature_id), "User Auth")
//!     .await?;
//!
//! // List checkpoints
//! let checkpoints = manager.list_checkpoints(project_id).await?;
//!
//! // Verify checkpoint integrity
//! manager.verify_checkpoint(&checkpoint)?;
//! ```

pub mod checkpoint;
pub mod manager;
pub mod repository;
pub mod signing;

// Re-export main types
pub use checkpoint::{
    Checkpoint, CheckpointInfo, FeatureSnapshot, GeneratedCodeSnapshot, MessageSnapshot,
    PhaseSnapshot, SnapshotData,
};
pub use manager::{
    CheckpointConfig, CheckpointManager, CheckpointStats, DEFAULT_MAX_PER_PROJECT,
    DEFAULT_RETENTION_DAYS, compute_content_hash,
};
pub use repository::CheckpointRepository;
pub use signing::{
    CheckpointSigner, CheckpointVerifier, PRIVATE_KEY_SIZE, PUBLIC_KEY_SIZE, SIGNATURE_SIZE,
    SigningError,
};
