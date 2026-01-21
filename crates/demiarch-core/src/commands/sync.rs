//! Sync commands for SQLite <-> JSONL export/import
//!
//! These commands provide git-friendly synchronization of the SQLite database
//! by exporting to JSONL files that can be committed to the repository.
//!
//! # Commands
//!
//! - `flush`: Export SQLite database to JSONL files
//! - `import`: Import JSONL files back into SQLite
//! - `status`: Check sync status (dirty state, pending changes)

use sqlx::SqlitePool;
use std::path::Path;

use crate::Result;
use crate::storage::jsonl;

// Re-export types for convenience
pub use jsonl::{ExportResult, ImportResult, SyncStatus};

/// Export SQLite database to JSONL files in the sync directory
///
/// This creates/updates JSONL files in `.demiarch/sync/` with one file per table.
/// The format is designed for clean git diffs - one JSON object per line.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `project_dir` - Root directory of the project (where .demiarch/ lives)
///
/// # Returns
/// Export result containing metadata and list of files written
///
/// # Example
/// ```ignore
/// let result = sync::flush(db.pool(), project_dir).await?;
/// println!("Exported {} records to {:?}", result.metadata.total_records, result.sync_dir);
/// ```
pub async fn flush(pool: &SqlitePool, project_dir: &Path) -> Result<ExportResult> {
    jsonl::export_to_jsonl(pool, project_dir).await
}

/// Import JSONL files from the sync directory into the database
///
/// This reads JSONL files from `.demiarch/sync/` and imports them into SQLite,
/// using INSERT OR REPLACE to handle both new records and updates.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `project_dir` - Root directory of the project (where .demiarch/ lives)
///
/// # Returns
/// Import result containing counts per table and any warnings
///
/// # Example
/// ```ignore
/// let result = sync::import(db.pool(), project_dir).await?;
/// println!("Imported {} records", result.total_records);
/// for warning in &result.warnings {
///     eprintln!("Warning: {}", warning);
/// }
/// ```
pub async fn import(pool: &SqlitePool, project_dir: &Path) -> Result<ImportResult> {
    jsonl::import_from_jsonl(pool, project_dir).await
}

/// Get the current sync status
///
/// Compares the database state with the last export to determine if there are
/// pending changes that need to be flushed.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `project_dir` - Root directory of the project (where .demiarch/ lives)
///
/// # Returns
/// Sync status including dirty flag, last sync time, and pending change count
///
/// # Example
/// ```ignore
/// let status = sync::status(db.pool(), project_dir).await?;
/// if status.dirty {
///     println!("{} changes pending", status.pending_changes);
/// } else {
///     println!("Up to date");
/// }
/// ```
pub async fn status(pool: &SqlitePool, project_dir: &Path) -> Result<SyncStatus> {
    jsonl::check_sync_status(pool, project_dir).await
}
