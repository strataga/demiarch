//! Application shutdown handler for graceful session termination
//!
//! Provides centralized shutdown management ensuring all resources are properly
//! released in the correct order when the application terminates.
//!
//! # Resource Cleanup Order
//!
//! 1. Pause active session (prevents new events)
//! 2. Release all held locks
//! 3. Process pending lock releases
//! 4. Optionally run cleanup operations
//! 5. Close database connections
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::session::{SessionManager, ShutdownHandler, ShutdownConfig};
//! use demiarch_core::domain::locking::LockManager;
//!
//! let handler = ShutdownHandler::new(
//!     session_manager,
//!     lock_manager,
//!     database,
//!     ShutdownConfig::default(),
//! );
//!
//! // On application exit
//! let result = handler.shutdown_gracefully().await?;
//! println!("Shutdown complete: {}", result.summary());
//! ```

use super::manager::SessionManager;
use super::session::SessionStatus;
use crate::domain::locking::LockManager;
use crate::error::{Error, Result};
use crate::storage::Database;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for shutdown behavior
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Whether to pause the active session (default: true)
    pub pause_session: bool,

    /// Whether to run cleanup operations during shutdown (default: false)
    /// Note: Cleanup can be slow for large databases
    pub run_cleanup: bool,

    /// Days threshold for session cleanup (only used if run_cleanup is true)
    pub cleanup_session_days: i64,

    /// Days threshold for event cleanup (only used if run_cleanup is true)
    pub cleanup_event_days: i64,

    /// Maximum time to wait for shutdown operations (default: 30 seconds)
    pub shutdown_timeout: Duration,

    /// Whether to force-release locks even if not owned by this process
    pub force_release_locks: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            pause_session: true,
            run_cleanup: false,
            cleanup_session_days: 30,
            cleanup_event_days: 30,
            shutdown_timeout: Duration::from_secs(30),
            force_release_locks: false,
        }
    }
}

impl ShutdownConfig {
    /// Create config that runs cleanup during shutdown
    pub fn with_cleanup(session_days: i64, event_days: i64) -> Self {
        Self {
            run_cleanup: true,
            cleanup_session_days: session_days,
            cleanup_event_days: event_days,
            ..Default::default()
        }
    }

    /// Create config for quick shutdown (no cleanup, minimal operations)
    pub fn quick() -> Self {
        Self {
            pause_session: true,
            run_cleanup: false,
            shutdown_timeout: Duration::from_secs(5),
            ..Default::default()
        }
    }

    /// Enable cleanup with the specified thresholds
    pub fn enable_cleanup(mut self, session_days: i64, event_days: i64) -> Self {
        self.run_cleanup = true;
        self.cleanup_session_days = session_days;
        self.cleanup_event_days = event_days;
        self
    }

    /// Set the shutdown timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Enable force release of locks
    pub fn force_locks(mut self) -> Self {
        self.force_release_locks = true;
        self
    }
}

/// Result of a shutdown operation
#[derive(Debug, Clone)]
pub struct ShutdownResult {
    /// Whether the session was paused
    pub session_paused: bool,

    /// The session ID that was paused (if any)
    pub session_id: Option<Uuid>,

    /// Number of locks released
    pub locks_released: u32,

    /// Number of sessions cleaned up (if cleanup was run)
    pub sessions_cleaned: u64,

    /// Number of events cleaned up (if cleanup was run)
    pub events_cleaned: u64,

    /// Whether the database was closed
    pub database_closed: bool,

    /// Any errors that occurred (non-fatal)
    pub warnings: Vec<String>,
}

impl ShutdownResult {
    /// Create a new empty result
    fn new() -> Self {
        Self {
            session_paused: false,
            session_id: None,
            locks_released: 0,
            sessions_cleaned: 0,
            events_cleaned: 0,
            database_closed: false,
            warnings: Vec::new(),
        }
    }

    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if self.session_paused {
            if let Some(id) = self.session_id {
                parts.push(format!("Session {} paused", id));
            } else {
                parts.push("Session paused".to_string());
            }
        }

        if self.locks_released > 0 {
            parts.push(format!("{} locks released", self.locks_released));
        }

        if self.sessions_cleaned > 0 || self.events_cleaned > 0 {
            parts.push(format!(
                "Cleaned {} sessions, {} events",
                self.sessions_cleaned, self.events_cleaned
            ));
        }

        if self.database_closed {
            parts.push("Database closed".to_string());
        }

        if parts.is_empty() {
            "Shutdown completed (no actions needed)".to_string()
        } else {
            parts.join("; ")
        }
    }

    /// Check if there were any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Handler for graceful application shutdown
///
/// This handler coordinates the shutdown of all session-related resources,
/// ensuring proper cleanup and state persistence.
#[derive(Debug)]
pub struct ShutdownHandler {
    session_manager: SessionManager,
    lock_manager: Arc<LockManager>,
    database: Database,
    config: ShutdownConfig,
}

impl ShutdownHandler {
    /// Create a new shutdown handler
    pub fn new(
        session_manager: SessionManager,
        lock_manager: Arc<LockManager>,
        database: Database,
        config: ShutdownConfig,
    ) -> Self {
        Self {
            session_manager,
            lock_manager,
            database,
            config,
        }
    }

    /// Create a shutdown handler with default configuration
    pub fn with_defaults(
        session_manager: SessionManager,
        lock_manager: Arc<LockManager>,
        database: Database,
    ) -> Self {
        Self::new(
            session_manager,
            lock_manager,
            database,
            ShutdownConfig::default(),
        )
    }

    /// Perform graceful shutdown
    ///
    /// This method:
    /// 1. Pauses the active session (if configured)
    /// 2. Releases all locks held by this process
    /// 3. Runs cleanup operations (if configured)
    /// 4. Closes the database connection
    ///
    /// Non-fatal errors are collected as warnings rather than failing the shutdown.
    pub async fn shutdown_gracefully(&self) -> Result<ShutdownResult> {
        let mut result = ShutdownResult::new();

        info!("Starting graceful shutdown");

        // Step 1: Pause active session
        if self.config.pause_session {
            match self.pause_active_session().await {
                Ok(Some(session_id)) => {
                    result.session_paused = true;
                    result.session_id = Some(session_id);
                    debug!(session_id = %session_id, "Active session paused");
                }
                Ok(None) => {
                    debug!("No active session to pause");
                }
                Err(e) => {
                    let warning = format!("Failed to pause session: {}", e);
                    warn!("{}", warning);
                    result.warnings.push(warning);
                }
            }
        }

        // Step 2: Release all locks
        match self.release_all_locks().await {
            Ok(count) => {
                result.locks_released = count;
                if count > 0 {
                    debug!(count = count, "Locks released");
                }
            }
            Err(e) => {
                let warning = format!("Failed to release locks: {}", e);
                warn!("{}", warning);
                result.warnings.push(warning);
            }
        }

        // Step 3: Run cleanup if configured
        if self.config.run_cleanup {
            match self.run_cleanup().await {
                Ok((sessions, events)) => {
                    result.sessions_cleaned = sessions;
                    result.events_cleaned = events;
                    if sessions > 0 || events > 0 {
                        debug!(sessions = sessions, events = events, "Cleanup completed");
                    }
                }
                Err(e) => {
                    let warning = format!("Cleanup failed: {}", e);
                    warn!("{}", warning);
                    result.warnings.push(warning);
                }
            }
        }

        // Step 4: Close database connection
        self.database.close().await;
        result.database_closed = true;
        debug!("Database connection closed");

        info!(
            session_paused = result.session_paused,
            locks_released = result.locks_released,
            "Graceful shutdown completed"
        );

        Ok(result)
    }

    /// Pause the active session if one exists
    async fn pause_active_session(&self) -> Result<Option<Uuid>> {
        let active = self.session_manager.get_active().await?;

        if let Some(session) = active {
            if session.status == SessionStatus::Active {
                self.session_manager.pause(session.id).await?;
                return Ok(Some(session.id));
            }
        }

        Ok(None)
    }

    /// Release all locks held by this process
    async fn release_all_locks(&self) -> Result<u32> {
        // Process any pending releases first
        self.lock_manager.process_releases().await;

        // Get list of active locks
        let active_locks = self.lock_manager.list_active_locks().await;
        let mut released = 0;

        for lock_info in active_locks {
            if lock_info.is_held_by_self() || self.config.force_release_locks {
                if let Err(e) = self.lock_manager.release_lock(lock_info.id).await {
                    warn!(
                        lock_id = %lock_info.id,
                        error = %e,
                        "Failed to release lock"
                    );
                } else {
                    released += 1;
                }
            }
        }

        // Process any new releases
        self.lock_manager.process_releases().await;

        Ok(released)
    }

    /// Run cleanup operations
    async fn run_cleanup(&self) -> Result<(u64, u64)> {
        let summary = self
            .session_manager
            .full_cleanup(
                self.config.cleanup_session_days,
                Some(self.config.cleanup_event_days),
            )
            .await?;

        Ok((summary.sessions_deleted, summary.events_deleted))
    }

    /// Quick shutdown that only pauses the session
    ///
    /// Use this for emergency shutdowns where time is critical.
    pub async fn shutdown_quick(&self) -> Result<ShutdownResult> {
        let mut result = ShutdownResult::new();

        // Only pause the session
        if let Ok(Some(session_id)) = self.pause_active_session().await {
            result.session_paused = true;
            result.session_id = Some(session_id);
        }

        // Close database
        self.database.close().await;
        result.database_closed = true;

        Ok(result)
    }

    /// End the session completely (marks as completed, not paused)
    ///
    /// Use this when the user explicitly ends their work session.
    pub async fn end_session(&self) -> Result<ShutdownResult> {
        let mut result = ShutdownResult::new();

        // Complete the active session instead of pausing
        if let Some(session) = self.session_manager.get_active().await? {
            self.session_manager.complete(session.id).await?;
            result.session_id = Some(session.id);
        }

        // Release locks
        match self.release_all_locks().await {
            Ok(count) => {
                result.locks_released = count;
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("Failed to release locks: {}", e));
            }
        }

        // Close database
        self.database.close().await;
        result.database_closed = true;

        Ok(result)
    }

    /// Abandon the session (marks as abandoned, not completed)
    ///
    /// Use this when the session is being abandoned without proper completion.
    pub async fn abandon_session(&self) -> Result<ShutdownResult> {
        let mut result = ShutdownResult::new();

        // Abandon the active session
        if let Some(session) = self.session_manager.get_active().await? {
            self.session_manager.abandon(session.id).await?;
            result.session_id = Some(session.id);
        }

        // Release locks
        match self.release_all_locks().await {
            Ok(count) => {
                result.locks_released = count;
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("Failed to release locks: {}", e));
            }
        }

        // Close database
        self.database.close().await;
        result.database_closed = true;

        Ok(result)
    }
}

/// Builder for creating a shutdown handler with custom configuration
pub struct ShutdownHandlerBuilder {
    session_manager: Option<SessionManager>,
    lock_manager: Option<Arc<LockManager>>,
    database: Option<Database>,
    config: ShutdownConfig,
}

impl ShutdownHandlerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            session_manager: None,
            lock_manager: None,
            database: None,
            config: ShutdownConfig::default(),
        }
    }

    /// Set the session manager
    pub fn session_manager(mut self, manager: SessionManager) -> Self {
        self.session_manager = Some(manager);
        self
    }

    /// Set the lock manager
    pub fn lock_manager(mut self, manager: Arc<LockManager>) -> Self {
        self.lock_manager = Some(manager);
        self
    }

    /// Set the database
    pub fn database(mut self, db: Database) -> Self {
        self.database = Some(db);
        self
    }

    /// Set the shutdown configuration
    pub fn config(mut self, config: ShutdownConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable cleanup during shutdown
    pub fn with_cleanup(mut self, session_days: i64, event_days: i64) -> Self {
        self.config = self.config.enable_cleanup(session_days, event_days);
        self
    }

    /// Set the shutdown timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config = self.config.timeout(timeout);
        self
    }

    /// Build the shutdown handler
    pub fn build(self) -> Result<ShutdownHandler> {
        let session_manager = self
            .session_manager
            .ok_or_else(|| Error::InvalidInput("Session manager is required".to_string()))?;
        let lock_manager = self
            .lock_manager
            .ok_or_else(|| Error::InvalidInput("Lock manager is required".to_string()))?;
        let database = self
            .database
            .ok_or_else(|| Error::InvalidInput("Database is required".to_string()))?;

        Ok(ShutdownHandler::new(
            session_manager,
            lock_manager,
            database,
            self.config,
        ))
    }
}

impl Default for ShutdownHandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locking::LockConfig;
    use tempfile::TempDir;

    async fn create_test_components() -> (SessionManager, Arc<LockManager>, Database, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");
        let session_manager = SessionManager::new(db.pool().clone());
        let lock_config = LockConfig::default().with_lock_dir(temp_dir.path().join("locks"));
        let lock_manager = Arc::new(LockManager::new(lock_config));
        lock_manager
            .initialize()
            .await
            .expect("Failed to initialize lock manager");

        (session_manager, lock_manager, db, temp_dir)
    }

    #[tokio::test]
    async fn test_shutdown_no_active_session() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;
        let handler = ShutdownHandler::with_defaults(session_manager, lock_manager, db);

        let result = handler
            .shutdown_gracefully()
            .await
            .expect("Shutdown failed");

        assert!(!result.session_paused);
        assert!(result.session_id.is_none());
        assert!(result.database_closed);
        assert!(result.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_shutdown_pauses_active_session() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        // Create an active session
        let session = session_manager
            .create(None, None, Some("Test session".to_string()))
            .await
            .expect("Failed to create session");

        // Verify session is initially active
        let initial = session_manager
            .get(session.id)
            .await
            .expect("Failed to get session")
            .expect("Session not found");
        assert!(initial.is_active());

        let handler = ShutdownHandler::with_defaults(session_manager, lock_manager, db);
        let result = handler
            .shutdown_gracefully()
            .await
            .expect("Shutdown failed");

        assert!(result.session_paused);
        assert_eq!(result.session_id, Some(session.id));
        // Note: Can't verify session state after shutdown since database is closed
        // The pausing happens before database close, so if we got here without error,
        // the session was successfully paused
    }

    #[tokio::test]
    async fn test_shutdown_releases_locks() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        // Acquire some locks
        let _guard1 = lock_manager
            .acquire_project_lock(Uuid::new_v4(), "test", None)
            .await
            .expect("Failed to acquire lock");
        let _guard2 = lock_manager
            .acquire_session_lock(Uuid::new_v4(), "test", None)
            .await
            .expect("Failed to acquire lock");

        let handler = ShutdownHandler::with_defaults(session_manager, lock_manager.clone(), db);
        let result = handler
            .shutdown_gracefully()
            .await
            .expect("Shutdown failed");

        // Locks should be released
        assert_eq!(result.locks_released, 2);
    }

    #[tokio::test]
    async fn test_shutdown_with_cleanup() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        let config = ShutdownConfig::with_cleanup(30, 30);
        let handler = ShutdownHandler::new(session_manager, lock_manager, db, config);

        let result = handler
            .shutdown_gracefully()
            .await
            .expect("Shutdown failed");

        // Cleanup should have run (even if nothing was cleaned)
        assert!(result.database_closed);
    }

    #[tokio::test]
    async fn test_quick_shutdown() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        // Create an active session
        session_manager
            .create(None, None, None)
            .await
            .expect("Failed to create session");

        let handler = ShutdownHandler::with_defaults(session_manager, lock_manager, db);
        let result = handler
            .shutdown_quick()
            .await
            .expect("Quick shutdown failed");

        assert!(result.session_paused);
        assert!(result.database_closed);
        assert_eq!(result.locks_released, 0); // Quick shutdown skips lock release
    }

    #[tokio::test]
    async fn test_end_session() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        // Create an active session
        let session = session_manager
            .create(None, None, None)
            .await
            .expect("Failed to create session");

        // Verify session is initially active
        let initial = session_manager
            .get(session.id)
            .await
            .expect("Failed to get session")
            .expect("Session not found");
        assert!(initial.is_active());

        let handler = ShutdownHandler::with_defaults(session_manager, lock_manager, db);
        let result = handler.end_session().await.expect("End session failed");

        assert_eq!(result.session_id, Some(session.id));
        // Note: Can't verify session state after shutdown since database is closed
        // The completing happens before database close, so if we got here without error,
        // the session was successfully completed
    }

    #[tokio::test]
    async fn test_abandon_session() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        // Create an active session
        let session = session_manager
            .create(None, None, None)
            .await
            .expect("Failed to create session");

        // Verify session is initially active
        let initial = session_manager
            .get(session.id)
            .await
            .expect("Failed to get session")
            .expect("Session not found");
        assert!(initial.is_active());

        let handler = ShutdownHandler::with_defaults(session_manager, lock_manager, db);
        let result = handler
            .abandon_session()
            .await
            .expect("Abandon session failed");

        assert_eq!(result.session_id, Some(session.id));
        // Note: Can't verify session state after shutdown since database is closed
        // The abandoning happens before database close, so if we got here without error,
        // the session was successfully abandoned
    }

    #[tokio::test]
    async fn test_shutdown_config_builder() {
        let config = ShutdownConfig::default()
            .enable_cleanup(60, 90)
            .timeout(Duration::from_secs(10))
            .force_locks();

        assert!(config.run_cleanup);
        assert_eq!(config.cleanup_session_days, 60);
        assert_eq!(config.cleanup_event_days, 90);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
        assert!(config.force_release_locks);
    }

    #[tokio::test]
    async fn test_shutdown_handler_builder() {
        let (session_manager, lock_manager, db, _temp) = create_test_components().await;

        let handler = ShutdownHandlerBuilder::new()
            .session_manager(session_manager)
            .lock_manager(lock_manager)
            .database(db)
            .with_cleanup(30, 30)
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to build handler");

        assert!(handler.config.run_cleanup);
        assert_eq!(handler.config.shutdown_timeout, Duration::from_secs(15));
    }

    #[tokio::test]
    async fn test_shutdown_result_summary() {
        let mut result = ShutdownResult::new();
        assert_eq!(result.summary(), "Shutdown completed (no actions needed)");

        result.session_paused = true;
        result.session_id = Some(Uuid::new_v4());
        result.locks_released = 3;
        result.database_closed = true;

        let summary = result.summary();
        assert!(summary.contains("Session"));
        assert!(summary.contains("paused"));
        assert!(summary.contains("3 locks released"));
        assert!(summary.contains("Database closed"));
    }
}
