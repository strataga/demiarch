//! Storage layer - SQLite + JSONL export
//!
//! Provides database management and migrations for demiarch.
//!
//! # Architecture
//!
//! - `database`: Connection pool management and initialization
//! - `migrations`: Schema versioning and automatic migration
//! - `jsonl`: JSONL export format for git-based synchronization
//!
//! # Usage
//!
//! ```ignore
//! use demiarch_core::storage::{Database, DatabaseManager};
//!
//! // Create an in-memory database for testing
//! let db = Database::in_memory().await?;
//!
//! // Or use the database manager for production
//! let manager = DatabaseManager::new().await?;
//! let global = manager.global();
//! ```

pub mod database;
pub mod jsonl;
pub mod migrations;

// Re-export commonly used types
pub use database::{Database, DatabaseConfig, DatabaseManager};
pub use migrations::{run_migrations, migration_status, MigrationStatus, CURRENT_VERSION};
