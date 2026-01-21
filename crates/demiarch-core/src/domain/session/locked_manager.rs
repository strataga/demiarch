//! Locked session manager for concurrent access
//!
//! Provides a thread-safe wrapper around SessionManager that acquires
//! locks before performing operations. This prevents race conditions
//! when multiple processes or threads access sessions simultaneously.

use super::event::SessionEvent;
use super::manager::SessionManager;
use super::session::{Session, SessionInfo, SessionPhase, SessionStatus};
use crate::domain::locking::{LockConfig, LockManager, SessionLockGuard};
use crate::error::{Error, Result};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Default timeout for session lock acquisition
const DEFAULT_SESSION_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// Locked session manager for concurrent access
///
/// This manager wraps the standard SessionManager and acquires locks
/// before performing any state-changing operations. It ensures that
/// only one process can modify a session at a time.
#[derive(Debug, Clone)]
pub struct LockedSessionManager {
    /// Underlying session manager
    inner: SessionManager,

    /// Lock manager for session locks
    lock_manager: Arc<LockManager>,

    /// Default timeout for lock acquisition
    lock_timeout: Duration,
}

impl LockedSessionManager {
    /// Create a new locked session manager
    pub fn new(pool: SqlitePool, lock_manager: Arc<LockManager>) -> Self {
        Self {
            inner: SessionManager::new(pool),
            lock_manager,
            lock_timeout: DEFAULT_SESSION_LOCK_TIMEOUT,
        }
    }

    /// Create a locked session manager with a custom lock directory
    pub fn with_lock_dir(pool: SqlitePool, lock_dir: impl Into<std::path::PathBuf>) -> Self {
        let config = LockConfig::default().with_lock_dir(lock_dir);
        let lock_manager = Arc::new(LockManager::new(config));
        Self::new(pool, lock_manager)
    }

    /// Set the default lock timeout
    pub fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = timeout;
        self
    }

    /// Get the underlying session manager
    pub fn inner(&self) -> &SessionManager {
        &self.inner
    }

    /// Get the lock manager
    pub fn lock_manager(&self) -> &LockManager {
        &self.lock_manager
    }

    /// Initialize the lock manager (create lock directory)
    pub async fn initialize(&self) -> Result<()> {
        self.lock_manager
            .initialize()
            .await
            .map_err(|e| Error::Other(format!("Failed to initialize lock manager: {}", e)))
    }

    // ========== Session Lifecycle (with locking) ==========

    /// Create a new session with locking
    ///
    /// This operation acquires a workspace-level lock to ensure
    /// atomicity when pausing existing sessions and creating new ones.
    pub async fn create(
        &self,
        project_id: Option<Uuid>,
        feature_id: Option<Uuid>,
        description: Option<String>,
    ) -> Result<Session> {
        // For session creation, we need to lock at the workspace level
        // to prevent race conditions with other session creations
        let _lock = self
            .lock_manager
            .acquire_resource_lock(
                crate::domain::locking::ResourceType::Workspace,
                "session-create",
                "session-manager:create",
                Some(self.lock_timeout),
            )
            .await
            .map_err(|e| Error::LockTimeout(format!("session-create: {}", e)))?;

        self.inner.create(project_id, feature_id, description).await
    }

    /// Get a session by ID (read-only, no lock needed)
    pub async fn get(&self, session_id: Uuid) -> Result<Option<Session>> {
        self.inner.get(session_id).await
    }

    /// Get the current active session (read-only, no lock needed)
    pub async fn get_active(&self) -> Result<Option<Session>> {
        self.inner.get_active().await
    }

    /// Get or create a session with locking
    pub async fn get_or_create(
        &self,
        project_id: Option<Uuid>,
        description: Option<String>,
    ) -> Result<Session> {
        // Acquire workspace lock for atomic get-or-create
        let _lock = self
            .lock_manager
            .acquire_resource_lock(
                crate::domain::locking::ResourceType::Workspace,
                "session-create",
                "session-manager:get-or-create",
                Some(self.lock_timeout),
            )
            .await
            .map_err(|e| Error::LockTimeout(format!("session-get-or-create: {}", e)))?;

        self.inner.get_or_create(project_id, description).await
    }

    /// Pause a session with locking
    pub async fn pause(&self, session_id: Uuid) -> Result<Session> {
        let _lock = self.acquire_session_lock(session_id, "pause").await?;
        self.inner.pause(session_id).await
    }

    /// Resume a session with locking
    ///
    /// Acquires locks on both the workspace (to pause other sessions)
    /// and the target session.
    pub async fn resume(&self, session_id: Uuid) -> Result<Session> {
        // Acquire workspace lock first (higher priority)
        let _workspace_lock = self
            .lock_manager
            .acquire_resource_lock(
                crate::domain::locking::ResourceType::Workspace,
                "session-resume",
                "session-manager:resume",
                Some(self.lock_timeout),
            )
            .await
            .map_err(|e| Error::LockTimeout(format!("session-resume-workspace: {}", e)))?;

        // Then acquire session lock
        let _session_lock = self.acquire_session_lock(session_id, "resume").await?;

        self.inner.resume(session_id).await
    }

    /// Complete a session with locking
    pub async fn complete(&self, session_id: Uuid) -> Result<Session> {
        let _lock = self.acquire_session_lock(session_id, "complete").await?;
        self.inner.complete(session_id).await
    }

    /// Abandon a session with locking
    pub async fn abandon(&self, session_id: Uuid) -> Result<Session> {
        let _lock = self.acquire_session_lock(session_id, "abandon").await?;
        self.inner.abandon(session_id).await
    }

    // ========== Context Switching (with locking) ==========

    /// Switch the current project in a session with locking
    pub async fn switch_project(
        &self,
        session_id: Uuid,
        project_id: Option<Uuid>,
    ) -> Result<Session> {
        let _lock = self
            .acquire_session_lock(session_id, "switch-project")
            .await?;
        self.inner.switch_project(session_id, project_id).await
    }

    /// Switch the current feature in a session with locking
    pub async fn switch_feature(
        &self,
        session_id: Uuid,
        feature_id: Option<Uuid>,
    ) -> Result<Session> {
        let _lock = self
            .acquire_session_lock(session_id, "switch-feature")
            .await?;
        self.inner.switch_feature(session_id, feature_id).await
    }

    /// Update the session phase with locking
    pub async fn set_phase(&self, session_id: Uuid, phase: SessionPhase) -> Result<Session> {
        let _lock = self.acquire_session_lock(session_id, "set-phase").await?;
        self.inner.set_phase(session_id, phase).await
    }

    /// Record a checkpoint in the session with locking
    pub async fn record_checkpoint(
        &self,
        session_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<Session> {
        let _lock = self
            .acquire_session_lock(session_id, "record-checkpoint")
            .await?;
        self.inner.record_checkpoint(session_id, checkpoint_id).await
    }

    /// Record an error in the session (no lock needed for append-only)
    pub async fn record_error(
        &self,
        session_id: Uuid,
        error_message: &str,
        error_code: Option<&str>,
    ) -> Result<()> {
        // Error recording is append-only, so we don't need a lock
        self.inner
            .record_error(session_id, error_message, error_code)
            .await
    }

    /// Touch a session to update last activity
    pub async fn touch(&self, session_id: Uuid) -> Result<Session> {
        // Touch is a simple timestamp update, lock for consistency
        let _lock = self.acquire_session_lock(session_id, "touch").await?;
        self.inner.touch(session_id).await
    }

    // ========== Listing and Queries (read-only, no locks) ==========

    /// List all sessions
    pub async fn list(&self, limit: Option<i32>) -> Result<Vec<SessionInfo>> {
        self.inner.list(limit).await
    }

    /// List sessions by status
    pub async fn list_by_status(&self, status: SessionStatus) -> Result<Vec<SessionInfo>> {
        self.inner.list_by_status(status).await
    }

    /// List sessions for a project
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<SessionInfo>> {
        self.inner.list_by_project(project_id).await
    }

    /// Get session events
    pub async fn get_events(
        &self,
        session_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<SessionEvent>> {
        self.inner.get_events(session_id, limit).await
    }

    // ========== Cleanup (with locking) ==========

    /// Delete a session with locking
    pub async fn delete(&self, session_id: Uuid) -> Result<bool> {
        let _lock = self.acquire_session_lock(session_id, "delete").await?;
        self.inner.delete(session_id).await
    }

    /// Cleanup old sessions with workspace lock
    pub async fn cleanup_old_sessions(&self, days: i64) -> Result<u64> {
        let _lock = self
            .lock_manager
            .acquire_resource_lock(
                crate::domain::locking::ResourceType::Workspace,
                "session-cleanup",
                "session-manager:cleanup",
                Some(self.lock_timeout),
            )
            .await
            .map_err(|e| Error::LockTimeout(format!("session-cleanup: {}", e)))?;

        self.inner.cleanup_old_sessions(days).await
    }

    /// Get session statistics (read-only, no lock needed)
    pub async fn stats(&self) -> Result<super::manager::SessionStats> {
        self.inner.stats().await
    }

    // ========== Lock Management ==========

    /// Acquire a lock on a session
    ///
    /// Returns a lock guard that will release the lock when dropped.
    pub async fn acquire_session_lock(
        &self,
        session_id: Uuid,
        operation: &str,
    ) -> Result<SessionLockGuard> {
        self.lock_manager
            .acquire_session_lock(
                session_id,
                &format!("session-manager:{}", operation),
                Some(self.lock_timeout),
            )
            .await
            .map_err(|e| Error::LockTimeout(format!("session-{}: {}", operation, e)))
    }

    /// Try to acquire a session lock without waiting
    pub async fn try_acquire_session_lock(
        &self,
        session_id: Uuid,
        operation: &str,
    ) -> Result<SessionLockGuard> {
        self.lock_manager
            .acquire_session_lock(session_id, &format!("session-manager:{}", operation), None)
            .await
            .map_err(|e| Error::LockTimeout(format!("session-{}: {}", operation, e)))
    }

    /// Execute an operation with a session lock held
    ///
    /// This is a convenience method that acquires a lock, runs the
    /// provided closure, and releases the lock when done.
    pub async fn with_session_lock<F, T>(
        &self,
        session_id: Uuid,
        operation: &str,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&SessionManager) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + '_>>
            + Send,
        T: Send,
    {
        let _lock = self.acquire_session_lock(session_id, operation).await?;
        f(&self.inner).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use tempfile::TempDir;

    async fn create_test_manager() -> (LockedSessionManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");

        let manager =
            LockedSessionManager::with_lock_dir(db.pool().clone(), temp_dir.path().join("locks"));
        manager.initialize().await.expect("Failed to initialize");

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_session_with_lock() {
        let (manager, _temp) = create_test_manager().await;

        let session = manager
            .create(None, None, Some("Test session".to_string()))
            .await
            .expect("Failed to create session");

        assert!(session.is_active());
        assert_eq!(session.description, Some("Test session".to_string()));
    }

    #[tokio::test]
    async fn test_pause_and_resume_with_locks() {
        let (manager, _temp) = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        assert!(session.is_active());

        // Pause
        let paused = manager.pause(session.id).await.unwrap();
        assert!(paused.is_paused());

        // Resume
        let resumed = manager.resume(session.id).await.unwrap();
        assert!(resumed.is_active());
    }

    #[tokio::test]
    async fn test_complete_session_with_lock() {
        let (manager, _temp) = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        let completed = manager.complete(session.id).await.unwrap();

        assert_eq!(completed.status, SessionStatus::Completed);
    }

    #[tokio::test]
    async fn test_concurrent_session_operations() {
        let (manager, _temp) = create_test_manager().await;
        let manager = Arc::new(manager);

        let session = manager.create(None, None, None).await.unwrap();

        // Spawn multiple concurrent operations
        let m1 = manager.clone();
        let m2 = manager.clone();
        let m3 = manager.clone();
        let sid = session.id;

        let handles = vec![
            tokio::spawn(async move { m1.touch(sid).await }),
            tokio::spawn(async move { m2.touch(sid).await }),
            tokio::spawn(async move { m3.touch(sid).await }),
        ];

        // All should succeed (with locking preventing race conditions)
        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_get_or_create_with_lock() {
        let (manager, _temp) = create_test_manager().await;

        // First call creates
        let session1 = manager.get_or_create(None, None).await.unwrap();

        // Second call returns same session
        let session2 = manager.get_or_create(None, None).await.unwrap();

        assert_eq!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_switch_project_with_lock() {
        let (manager, _temp) = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();

        // Switch to no project (clearing) - doesn't require existing project
        let updated = manager
            .switch_project(session.id, None)
            .await
            .unwrap();

        assert_eq!(updated.current_project_id, None);
    }

    #[tokio::test]
    async fn test_set_phase_with_lock() {
        let (manager, _temp) = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();

        let updated = manager
            .set_phase(session.id, SessionPhase::Building)
            .await
            .unwrap();

        assert_eq!(updated.phase, SessionPhase::Building);
    }

    #[tokio::test]
    async fn test_list_operations_no_lock() {
        let (manager, _temp) = create_test_manager().await;

        // Create some sessions
        let s1 = manager.create(None, None, None).await.unwrap();
        manager.complete(s1.id).await.unwrap();
        let _s2 = manager.create(None, None, None).await.unwrap();

        // List operations should work without locks
        let all = manager.list(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let completed = manager.list_by_status(SessionStatus::Completed).await.unwrap();
        assert_eq!(completed.len(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_with_workspace_lock() {
        let (manager, _temp) = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        manager.complete(session.id).await.unwrap();

        // Cleanup should acquire workspace lock
        // Use a large number of days so nothing gets cleaned up
        let cleaned = manager.cleanup_old_sessions(365).await.unwrap();
        // Sessions from today won't be cleaned up with 365 days threshold
        assert_eq!(cleaned, 0);
    }
}
