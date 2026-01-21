//! Sync commands for SQLite <-> JSONL export/import

use crate::Result;

/// Export SQLite to JSONL (flush)
pub async fn flush() -> Result<()> {
    Ok(())
}

/// Import JSONL to SQLite
pub async fn import() -> Result<()> {
    Ok(())
}

/// Get sync status
pub async fn status() -> Result<SyncStatus> {
    Ok(SyncStatus {
        dirty: false,
        last_sync_at: None,
        pending_changes: 0,
    })
}

#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub dirty: bool,
    pub last_sync_at: Option<String>,
    pub pending_changes: usize,
}
