//! Resource locking for concurrent access
//!
//! This module provides file and resource locking mechanisms to prevent
//! concurrent access conflicts in multi-project workspaces and multi-agent
//! scenarios.
//!
//! # Architecture
//!
//! - **Lock Types**: `ProjectLock`, `SessionLock`, `ResourceLock`
//! - **Lock Manager**: `LockManager` for coordinating locks across processes
//! - **Guards**: RAII-style lock guards for automatic release
//!
//! # Features
//!
//! - File-based distributed locking for cross-process safety
//! - Advisory locks for in-process coordination
//! - Timeout support with configurable wait times
//! - Automatic cleanup of stale locks
//! - Lock hierarchy to prevent deadlocks
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::locking::{LockManager, LockConfig, ResourceType};
//!
//! let manager = LockManager::new(LockConfig::default());
//!
//! // Acquire a project lock
//! let guard = manager.acquire_project_lock(project_id, Duration::from_secs(5)).await?;
//!
//! // Do work with the project...
//!
//! // Lock is automatically released when guard is dropped
//! ```

pub mod event;
pub mod guard;
pub mod manager;
pub mod types;

// Re-export main types
pub use event::{LockEvent, LockEventType};
pub use guard::{LockGuard, ProjectLockGuard, ResourceLockGuard, SessionLockGuard};
pub use manager::LockManager;
pub use types::{LockConfig, LockError, LockInfo, LockResult, LockStatus, ResourceType};
