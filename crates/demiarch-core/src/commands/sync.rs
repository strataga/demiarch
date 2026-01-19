//! Sync commands for SQLite <-> JSONL export/import

use crate::Result;

/// Export SQLite to JSONL (flush)
pub async fn flush() -> Result<()> {
    todo!("Implement SQLite -> JSONL export")
}

/// Import JSONL to SQLite
pub async fn import() -> Result<()> {
    todo!("Implement JSONL -> SQLite import")
}

/// Get sync status
pub async fn status() -> Result<SyncStatus> {
    todo!("Implement sync status")
}

#[derive(Debug)]
pub struct SyncStatus {
    pub dirty: bool,
    pub last_sync_at: Option<String>,
    pub pending_changes: usize,
}
