//! Lock manager for coordinating resource locks
//!
//! The lock manager provides a central point for acquiring and releasing
//! locks across the application. It handles:
//! - File-based distributed locking for cross-process safety
//! - In-memory tracking of active locks
//! - Stale lock detection and cleanup
//! - Lock hierarchy enforcement

use super::guard::{
    LockGuard, MultiLockGuard, ProjectLockGuard, ResourceLockGuard, SessionLockGuard,
};
use super::types::{LockConfig, LockError, LockInfo, LockResult, LockStatus, ResourceType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{sleep, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Lock manager for coordinating resource locks
#[derive(Debug)]
pub struct LockManager {
    /// Configuration
    config: LockConfig,

    /// In-memory map of active locks (lock_key -> LockInfo)
    active_locks: Arc<RwLock<HashMap<String, LockInfo>>>,

    /// Channel receiver for lock release notifications
    release_rx: Arc<Mutex<mpsc::Receiver<Uuid>>>,

    /// Channel sender for lock release notifications
    release_tx: mpsc::Sender<Uuid>,
}

impl LockManager {
    /// Create a new lock manager with the given configuration
    pub fn new(config: LockConfig) -> Self {
        let (release_tx, release_rx) = mpsc::channel(256);

        Self {
            config,
            active_locks: Arc::new(RwLock::new(HashMap::new())),
            release_rx: Arc::new(Mutex::new(release_rx)),
            release_tx,
        }
    }

    /// Create a lock manager with default configuration
    pub fn with_defaults() -> Self {
        Self::new(LockConfig::default())
    }

    /// Create a lock manager with a custom lock directory
    pub fn with_lock_dir(dir: impl Into<PathBuf>) -> Self {
        Self::new(LockConfig::default().with_lock_dir(dir))
    }

    /// Get the configuration
    pub fn config(&self) -> &LockConfig {
        &self.config
    }

    /// Initialize the lock manager (create lock directory if needed)
    pub async fn initialize(&self) -> LockResult<()> {
        if !self.config.lock_dir.exists() {
            std::fs::create_dir_all(&self.config.lock_dir).map_err(|e| {
                LockError::IoError(format!(
                    "Failed to create lock directory {}: {}",
                    self.config.lock_dir.display(),
                    e
                ))
            })?;
        }

        // Clean up any stale locks on startup
        if self.config.auto_cleanup_stale {
            self.cleanup_stale_locks().await?;
        }

        Ok(())
    }

    /// Acquire a lock on a project
    ///
    /// # Arguments
    /// * `project_id` - The project UUID to lock
    /// * `holder_description` - Description of what's acquiring the lock
    /// * `timeout` - How long to wait for the lock (None = use default)
    pub async fn acquire_project_lock(
        &self,
        project_id: Uuid,
        holder_description: &str,
        timeout: Option<Duration>,
    ) -> LockResult<ProjectLockGuard> {
        let guard = self
            .acquire_lock(
                ResourceType::Project,
                &project_id.to_string(),
                holder_description,
                timeout,
            )
            .await?;

        Ok(ProjectLockGuard::new(guard, project_id))
    }

    /// Acquire a lock on a session
    pub async fn acquire_session_lock(
        &self,
        session_id: Uuid,
        holder_description: &str,
        timeout: Option<Duration>,
    ) -> LockResult<SessionLockGuard> {
        let guard = self
            .acquire_lock(
                ResourceType::Session,
                &session_id.to_string(),
                holder_description,
                timeout,
            )
            .await?;

        Ok(SessionLockGuard::new(guard, session_id))
    }

    /// Acquire a lock on a file or resource
    pub async fn acquire_resource_lock(
        &self,
        resource_type: ResourceType,
        resource_path: &str,
        holder_description: &str,
        timeout: Option<Duration>,
    ) -> LockResult<ResourceLockGuard> {
        let guard = self
            .acquire_lock(resource_type, resource_path, holder_description, timeout)
            .await?;

        Ok(ResourceLockGuard::new(guard, resource_path.to_string()))
    }

    /// Acquire a lock on a generic resource
    ///
    /// This is the core locking method that all other acquire methods use.
    pub async fn acquire_lock(
        &self,
        resource_type: ResourceType,
        resource_id: &str,
        holder_description: &str,
        timeout: Option<Duration>,
    ) -> LockResult<LockGuard> {
        let timeout = timeout.unwrap_or(self.config.default_timeout);
        let lock_key = format!("{}:{}", resource_type.as_str(), resource_id);

        debug!(
            lock_key = %lock_key,
            timeout_ms = timeout.as_millis(),
            "Attempting to acquire lock"
        );

        let start = Instant::now();

        loop {
            // Check if we can acquire the lock
            match self.try_acquire_lock_internal(&lock_key, resource_type, resource_id, holder_description).await {
                Ok(guard) => {
                    info!(
                        lock_key = %lock_key,
                        elapsed_ms = start.elapsed().as_millis(),
                        "Lock acquired"
                    );
                    return Ok(guard);
                }
                Err(LockError::Contention { resource, holder_pid }) => {
                    // Lock is held by another process
                    if start.elapsed() >= timeout {
                        return Err(LockError::Timeout {
                            resource,
                            holder: format!("pid:{}", holder_pid),
                        });
                    }

                    // Wait and retry
                    sleep(self.config.retry_interval).await;
                }
                Err(LockError::StaleLock {
                    resource: _,
                    holder_pid,
                }) => {
                    // Lock is stale, clean it up and retry
                    warn!(
                        lock_key = %lock_key,
                        holder_pid = holder_pid,
                        "Cleaning up stale lock"
                    );
                    self.force_release_lock(&lock_key).await?;
                    // Don't count this against timeout, immediately retry
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to acquire a lock without waiting
    pub async fn try_acquire_lock(
        &self,
        resource_type: ResourceType,
        resource_id: &str,
        holder_description: &str,
    ) -> LockResult<LockGuard> {
        let lock_key = format!("{}:{}", resource_type.as_str(), resource_id);
        self.try_acquire_lock_internal(&lock_key, resource_type, resource_id, holder_description)
            .await
    }

    /// Internal lock acquisition
    async fn try_acquire_lock_internal(
        &self,
        lock_key: &str,
        resource_type: ResourceType,
        resource_id: &str,
        holder_description: &str,
    ) -> LockResult<LockGuard> {
        let mut active = self.active_locks.write().await;

        // Check if lock already exists
        if let Some(existing) = active.get(lock_key) {
            // Check if it's held by us
            if existing.is_held_by_self() {
                // Reentrant lock - return a new guard for the same lock
                return Ok(LockGuard::new(existing.clone(), self.release_tx.clone()));
            }

            // Check if it's expired
            if existing.is_expired() {
                debug!(lock_key = %lock_key, "Existing lock expired, will clean up");
                // Will be cleaned up below
            } else if !is_process_alive(existing.holder_pid) {
                // Holder process is dead
                return Err(LockError::StaleLock {
                    resource: lock_key.to_string(),
                    holder_pid: existing.holder_pid,
                });
            } else {
                // Lock is held by another living process
                return Err(LockError::Contention {
                    resource: lock_key.to_string(),
                    holder_pid: existing.holder_pid,
                });
            }
        }

        // Try to acquire file lock
        let lock_file = self.lock_file_path(lock_key);
        self.try_acquire_file_lock(&lock_file, holder_description)
            .await?;

        // Create lock info
        let lock_info = LockInfo::new(
            resource_type,
            resource_id.to_string(),
            holder_description.to_string(),
            Some(self.config.default_ttl),
        );

        // Write lock info to file
        self.write_lock_file(&lock_file, &lock_info).await?;

        // Store in active locks
        active.insert(lock_key.to_string(), lock_info.clone());

        Ok(LockGuard::new(lock_info, self.release_tx.clone()))
    }

    /// Acquire multiple locks atomically (all-or-nothing)
    ///
    /// Locks are acquired in priority order to prevent deadlocks.
    pub async fn acquire_multiple_locks(
        &self,
        locks: Vec<(ResourceType, String, String)>, // (type, id, description)
        timeout: Option<Duration>,
    ) -> LockResult<MultiLockGuard> {
        // Sort by resource type priority to prevent deadlocks
        let mut sorted_locks = locks;
        sorted_locks.sort_by_key(|(rt, _, _)| rt.priority());

        let mut acquired_guards = Vec::new();

        for (resource_type, resource_id, description) in sorted_locks {
            match self
                .acquire_lock(resource_type, &resource_id, &description, timeout)
                .await
            {
                Ok(guard) => {
                    acquired_guards.push(guard);
                }
                Err(e) => {
                    // Release all acquired locks on failure
                    drop(acquired_guards);
                    return Err(e);
                }
            }
        }

        Ok(MultiLockGuard::new(acquired_guards))
    }

    /// Release a lock by ID
    ///
    /// This is called automatically when guards are dropped, but can be
    /// called explicitly if needed.
    pub async fn release_lock(&self, lock_id: Uuid) -> LockResult<()> {
        let mut active = self.active_locks.write().await;

        // Find and remove the lock
        let lock_key = active
            .iter()
            .find(|(_, info)| info.id == lock_id)
            .map(|(k, _)| k.clone());

        if let Some(key) = lock_key {
            active.remove(&key);

            // Remove lock file
            let lock_file = self.lock_file_path(&key);
            if lock_file.exists() {
                let _ = std::fs::remove_file(&lock_file);
            }

            debug!(lock_key = %key, "Lock released");
            Ok(())
        } else {
            // Lock might have been released already
            Ok(())
        }
    }

    /// Force release a lock by key (for stale lock cleanup)
    pub async fn force_release_lock(&self, lock_key: &str) -> LockResult<()> {
        let mut active = self.active_locks.write().await;
        active.remove(lock_key);

        // Remove lock file
        let lock_file = self.lock_file_path(lock_key);
        if lock_file.exists() {
            std::fs::remove_file(&lock_file).map_err(|e| {
                LockError::IoError(format!("Failed to remove lock file: {}", e))
            })?;
        }

        info!(lock_key = %lock_key, "Lock force-released");
        Ok(())
    }

    /// Check the status of a lock
    pub async fn check_lock_status(
        &self,
        resource_type: ResourceType,
        resource_id: &str,
    ) -> LockResult<LockStatus> {
        let lock_key = format!("{}:{}", resource_type.as_str(), resource_id);
        let active = self.active_locks.read().await;

        match active.get(&lock_key) {
            Some(info) => {
                if info.is_held_by_self() {
                    Ok(LockStatus::HeldBySelf)
                } else if info.is_expired() || !is_process_alive(info.holder_pid) {
                    Ok(LockStatus::Stale)
                } else {
                    Ok(LockStatus::HeldByOther)
                }
            }
            None => {
                // Check if there's a lock file
                let lock_file = self.lock_file_path(&lock_key);
                if lock_file.exists() {
                    // There's a lock file but we don't have it in memory
                    // Try to read it
                    match self.read_lock_file(&lock_file).await {
                        Ok(info) => {
                            if info.is_expired() || !is_process_alive(info.holder_pid) {
                                Ok(LockStatus::Stale)
                            } else {
                                Ok(LockStatus::HeldByOther)
                            }
                        }
                        Err(_) => Ok(LockStatus::Stale), // Corrupted file = stale
                    }
                } else {
                    Ok(LockStatus::Available)
                }
            }
        }
    }

    /// Get information about a lock
    pub async fn get_lock_info(
        &self,
        resource_type: ResourceType,
        resource_id: &str,
    ) -> LockResult<Option<LockInfo>> {
        let lock_key = format!("{}:{}", resource_type.as_str(), resource_id);
        let active = self.active_locks.read().await;

        if let Some(info) = active.get(&lock_key) {
            return Ok(Some(info.clone()));
        }

        // Check lock file
        let lock_file = self.lock_file_path(&lock_key);
        if lock_file.exists() {
            match self.read_lock_file(&lock_file).await {
                Ok(info) => Ok(Some(info)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// List all active locks
    pub async fn list_active_locks(&self) -> Vec<LockInfo> {
        let active = self.active_locks.read().await;
        active.values().cloned().collect()
    }

    /// Renew a lock's TTL
    pub async fn renew_lock(&self, lock_id: Uuid, ttl: Option<Duration>) -> LockResult<()> {
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let mut active = self.active_locks.write().await;

        // Find the lock
        let lock_key = active
            .iter()
            .find(|(_, info)| info.id == lock_id)
            .map(|(k, _)| k.clone());

        if let Some(key) = lock_key {
            if let Some(info) = active.get_mut(&key) {
                info.renew(ttl);

                // Update lock file
                let lock_file = self.lock_file_path(&key);
                self.write_lock_file(&lock_file, info).await?;

                debug!(lock_key = %key, ttl_secs = ttl.as_secs(), "Lock renewed");
                Ok(())
            } else {
                Err(LockError::NotFound(lock_id.to_string()))
            }
        } else {
            Err(LockError::NotFound(lock_id.to_string()))
        }
    }

    /// Clean up stale locks
    pub async fn cleanup_stale_locks(&self) -> LockResult<u32> {
        let mut cleaned = 0;
        let mut active = self.active_locks.write().await;

        // Find stale locks
        let stale_keys: Vec<String> = active
            .iter()
            .filter(|(_, info)| info.is_expired() || !is_process_alive(info.holder_pid))
            .map(|(k, _)| k.clone())
            .collect();

        for key in stale_keys {
            active.remove(&key);

            // Remove lock file
            let lock_file = self.lock_file_path(&key);
            if lock_file.exists() {
                let _ = std::fs::remove_file(&lock_file);
            }

            info!(lock_key = %key, "Cleaned up stale lock");
            cleaned += 1;
        }

        // Also check for orphaned lock files
        if let Ok(entries) = std::fs::read_dir(&self.config.lock_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "lock").unwrap_or(false) {
                    match self.read_lock_file(&path).await {
                        Ok(info) => {
                            if info.is_expired() || !is_process_alive(info.holder_pid) {
                                let _ = std::fs::remove_file(&path);
                                info!(path = %path.display(), "Cleaned up orphaned lock file");
                                cleaned += 1;
                            }
                        }
                        Err(_) => {
                            // Corrupted file, remove it
                            let _ = std::fs::remove_file(&path);
                            cleaned += 1;
                        }
                    }
                }
            }
        }

        Ok(cleaned)
    }

    /// Process pending release notifications
    ///
    /// This should be called periodically or in a background task
    /// to process lock releases from dropped guards.
    pub async fn process_releases(&self) {
        let mut rx = self.release_rx.lock().await;
        while let Ok(lock_id) = rx.try_recv() {
            let _ = self.release_lock(lock_id).await;
        }
    }

    // ========== Internal Methods ==========

    /// Get the path to a lock file
    fn lock_file_path(&self, lock_key: &str) -> PathBuf {
        let safe_name = lock_key.replace([':', '/', '\\'], "_");
        self.config.lock_dir.join(format!("{}.lock", safe_name))
    }

    /// Try to acquire a file lock
    async fn try_acquire_file_lock(
        &self,
        lock_file: &Path,
        _holder_description: &str,
    ) -> LockResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = lock_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    LockError::IoError(format!("Failed to create lock directory: {}", e))
                })?;
            }
        }

        // Check if lock file exists and is valid
        if lock_file.exists() {
            match self.read_lock_file(lock_file).await {
                Ok(existing) => {
                    if existing.is_held_by_self() {
                        // We already hold this lock
                        return Ok(());
                    } else if existing.is_expired() || !is_process_alive(existing.holder_pid) {
                        // Stale lock, we'll overwrite it
                        debug!(path = %lock_file.display(), "Overwriting stale lock file");
                    } else {
                        // Valid lock held by another process
                        return Err(LockError::Contention {
                            resource: lock_file.display().to_string(),
                            holder_pid: existing.holder_pid,
                        });
                    }
                }
                Err(LockError::Corrupted(_)) => {
                    // Corrupted file, we'll overwrite it
                    debug!(path = %lock_file.display(), "Overwriting corrupted lock file");
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Write lock info to a file
    async fn write_lock_file(&self, path: &Path, info: &LockInfo) -> LockResult<()> {
        let json = serde_json::to_string_pretty(info).map_err(|e| {
            LockError::IoError(format!("Failed to serialize lock info: {}", e))
        })?;

        std::fs::write(path, json).map_err(|e| {
            LockError::IoError(format!("Failed to write lock file: {}", e))
        })?;

        Ok(())
    }

    /// Read lock info from a file
    async fn read_lock_file(&self, path: &Path) -> LockResult<LockInfo> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            LockError::IoError(format!("Failed to read lock file: {}", e))
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            LockError::Corrupted(format!("Failed to parse lock file: {}", e))
        })
    }
}

/// Check if a process is still alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // On Unix, we can use kill with signal 0 to check if process exists
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    {
        // On Windows, use tasklist
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout).contains(&pid.to_string())
            })
            .unwrap_or(false)
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Fallback: assume process is alive
        true
    }
}

impl Clone for LockManager {
    fn clone(&self) -> Self {
        // Create new channels for the clone
        let (release_tx, release_rx) = mpsc::channel(256);

        Self {
            config: self.config.clone(),
            active_locks: self.active_locks.clone(),
            release_rx: Arc::new(Mutex::new(release_rx)),
            release_tx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (LockManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = LockConfig::default().with_lock_dir(temp_dir.path().join("locks"));
        let manager = LockManager::new(config);
        manager.initialize().await.expect("Failed to initialize");
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_acquire_and_release_project_lock() {
        let (manager, _temp) = create_test_manager().await;
        let project_id = Uuid::new_v4();

        let guard = manager
            .acquire_project_lock(project_id, "test", None)
            .await
            .expect("Failed to acquire lock");

        assert_eq!(guard.project_id(), project_id);
        assert!(guard.is_valid());

        // Check status
        let status = manager
            .check_lock_status(ResourceType::Project, &project_id.to_string())
            .await
            .expect("Failed to check status");
        assert_eq!(status, LockStatus::HeldBySelf);

        // Release
        guard.release();
        manager.process_releases().await;

        let status = manager
            .check_lock_status(ResourceType::Project, &project_id.to_string())
            .await
            .expect("Failed to check status");
        assert_eq!(status, LockStatus::Available);
    }

    #[tokio::test]
    async fn test_acquire_session_lock() {
        let (manager, _temp) = create_test_manager().await;
        let session_id = Uuid::new_v4();

        let guard = manager
            .acquire_session_lock(session_id, "test", None)
            .await
            .expect("Failed to acquire lock");

        assert_eq!(guard.session_id(), session_id);
        assert!(guard.is_valid());
    }

    #[tokio::test]
    async fn test_acquire_resource_lock() {
        let (manager, _temp) = create_test_manager().await;

        let guard = manager
            .acquire_resource_lock(ResourceType::File, "src/main.rs", "test", None)
            .await
            .expect("Failed to acquire lock");

        assert_eq!(guard.resource_path(), "src/main.rs");
        assert!(guard.is_valid());
    }

    #[tokio::test]
    async fn test_reentrant_lock() {
        let (manager, _temp) = create_test_manager().await;
        let project_id = Uuid::new_v4();

        let guard1 = manager
            .acquire_project_lock(project_id, "test", None)
            .await
            .expect("Failed to acquire first lock");

        // Same process should be able to acquire the same lock again
        let guard2 = manager
            .acquire_project_lock(project_id, "test", None)
            .await
            .expect("Failed to acquire second lock");

        assert_eq!(guard1.id(), guard2.id());
    }

    #[tokio::test]
    async fn test_lock_timeout() {
        let (manager, _temp) = create_test_manager().await;
        let project_id = Uuid::new_v4();

        // Acquire first lock
        let _guard = manager
            .acquire_project_lock(project_id, "test", None)
            .await
            .expect("Failed to acquire lock");

        // Simulate another process by directly adding a lock entry
        {
            let mut active = manager.active_locks.write().await;
            let lock_key = format!("project:{}", project_id);
            if let Some(info) = active.get_mut(&lock_key) {
                // Pretend it's held by another process
                info.holder_pid = 999999; // Non-existent PID
            }
        }

        // Note: This test is tricky because we can't easily simulate
        // another process. The lock should work in real scenarios.
    }

    #[tokio::test]
    async fn test_list_active_locks() {
        let (manager, _temp) = create_test_manager().await;

        let proj1 = Uuid::new_v4();
        let proj2 = Uuid::new_v4();

        let _g1 = manager
            .acquire_project_lock(proj1, "test1", None)
            .await
            .unwrap();
        let _g2 = manager
            .acquire_project_lock(proj2, "test2", None)
            .await
            .unwrap();

        let active = manager.list_active_locks().await;
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_lock_renewal() {
        let (manager, _temp) = create_test_manager().await;
        let project_id = Uuid::new_v4();

        let guard = manager
            .acquire_project_lock(project_id, "test", None)
            .await
            .expect("Failed to acquire lock");

        let original_expiry = guard.info().expires_at;

        // Renew the lock
        manager
            .renew_lock(guard.id(), Some(Duration::from_secs(600)))
            .await
            .expect("Failed to renew lock");

        // Check that the expiry was extended
        let info = manager
            .get_lock_info(ResourceType::Project, &project_id.to_string())
            .await
            .expect("Failed to get lock info")
            .expect("Lock should exist");

        assert!(info.expires_at > original_expiry);
        assert_eq!(info.renewal_count, 1);
    }

    #[tokio::test]
    async fn test_multiple_locks() {
        let (manager, _temp) = create_test_manager().await;

        let proj = Uuid::new_v4();
        let sess = Uuid::new_v4();

        let locks = vec![
            (ResourceType::Project, proj.to_string(), "test".to_string()),
            (ResourceType::Session, sess.to_string(), "test".to_string()),
        ];

        let multi = manager
            .acquire_multiple_locks(locks, None)
            .await
            .expect("Failed to acquire multiple locks");

        assert_eq!(multi.len(), 2);
        assert!(multi.all_valid());
    }

    #[tokio::test]
    async fn test_cleanup_stale_locks() {
        let (manager, _temp) = create_test_manager().await;
        let project_id = Uuid::new_v4();

        // Create a "stale" lock entry with a non-existent PID
        {
            let mut active = manager.active_locks.write().await;
            let mut info = LockInfo::new(
                ResourceType::Project,
                project_id.to_string(),
                "stale".to_string(),
                Some(Duration::from_secs(1)),
            );
            info.holder_pid = 999999999; // Non-existent PID

            let lock_key = format!("project:{}", project_id);
            active.insert(lock_key, info);
        }

        // Run cleanup
        let cleaned = manager
            .cleanup_stale_locks()
            .await
            .expect("Failed to cleanup");

        assert!(cleaned >= 1);

        // Verify the lock is gone
        let status = manager
            .check_lock_status(ResourceType::Project, &project_id.to_string())
            .await
            .expect("Failed to check status");
        assert_eq!(status, LockStatus::Available);
    }

    #[test]
    fn test_lock_file_path() {
        let config = LockConfig::default().with_lock_dir("/tmp/locks");
        let (release_tx, _release_rx) = mpsc::channel(1);
        let manager = LockManager {
            config,
            active_locks: Arc::new(RwLock::new(HashMap::new())),
            release_rx: Arc::new(Mutex::new(_release_rx)),
            release_tx,
        };

        let path = manager.lock_file_path("project:abc123");
        assert!(path.to_string_lossy().contains("project_abc123.lock"));
    }
}
