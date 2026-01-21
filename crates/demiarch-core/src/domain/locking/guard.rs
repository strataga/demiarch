//! Lock guards for RAII-style lock management
//!
//! Guards automatically release locks when dropped, ensuring proper cleanup
//! even in the presence of panics or early returns.

use super::types::{LockInfo, ResourceType};
use std::fmt;
use std::ops::Deref;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Channel for notifying the lock manager when a guard is dropped
pub type ReleaseNotifier = mpsc::Sender<Uuid>;

/// A generic lock guard that holds a lock on a resource
///
/// The lock is automatically released when the guard is dropped.
#[derive(Debug)]
pub struct LockGuard {
    /// Information about the held lock
    info: LockInfo,

    /// Channel to notify the lock manager of release
    release_tx: Option<ReleaseNotifier>,

    /// Whether the lock has been explicitly released
    released: bool,
}

impl LockGuard {
    /// Create a new lock guard
    pub(crate) fn new(info: LockInfo, release_tx: ReleaseNotifier) -> Self {
        Self {
            info,
            release_tx: Some(release_tx),
            released: false,
        }
    }

    /// Create a lock guard without a release notifier (for testing)
    #[cfg(test)]
    pub fn new_test(info: LockInfo) -> Self {
        Self {
            info,
            release_tx: None,
            released: false,
        }
    }

    /// Get the lock ID
    pub fn id(&self) -> Uuid {
        self.info.id
    }

    /// Get the resource type
    pub fn resource_type(&self) -> ResourceType {
        self.info.resource_type
    }

    /// Get the resource ID
    pub fn resource_id(&self) -> &str {
        &self.info.resource_id
    }

    /// Get the lock info
    pub fn info(&self) -> &LockInfo {
        &self.info
    }

    /// Get mutable lock info (for renewal)
    #[allow(dead_code)]
    pub(crate) fn info_mut(&mut self) -> &mut LockInfo {
        &mut self.info
    }

    /// Check if the lock is still valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.released && !self.info.is_expired()
    }

    /// Explicitly release the lock (normally done automatically on drop)
    pub fn release(mut self) {
        self.do_release();
    }

    /// Internal release implementation
    fn do_release(&mut self) {
        if !self.released {
            self.released = true;
            if let Some(tx) = self.release_tx.take() {
                // Non-blocking send - if the receiver is gone, that's fine
                let _ = tx.try_send(self.info.id);
            }
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        self.do_release();
    }
}

impl fmt::Display for LockGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lock[{}:{}]",
            self.info.resource_type, self.info.resource_id
        )
    }
}

/// A guard for project-level locks
///
/// Provides a strongly-typed wrapper around LockGuard for project resources.
#[derive(Debug)]
pub struct ProjectLockGuard {
    inner: LockGuard,
    project_id: Uuid,
}

impl ProjectLockGuard {
    /// Create a new project lock guard
    pub(crate) fn new(guard: LockGuard, project_id: Uuid) -> Self {
        Self {
            inner: guard,
            project_id,
        }
    }

    /// Get the project ID
    pub fn project_id(&self) -> Uuid {
        self.project_id
    }

    /// Get the underlying lock guard
    pub fn guard(&self) -> &LockGuard {
        &self.inner
    }

    /// Check if the lock is still valid
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Release the lock explicitly
    pub fn release(self) {
        self.inner.release();
    }
}

impl Deref for ProjectLockGuard {
    type Target = LockGuard;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Display for ProjectLockGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProjectLock[{}]", self.project_id)
    }
}

/// A guard for session-level locks
#[derive(Debug)]
pub struct SessionLockGuard {
    inner: LockGuard,
    session_id: Uuid,
}

impl SessionLockGuard {
    /// Create a new session lock guard
    pub(crate) fn new(guard: LockGuard, session_id: Uuid) -> Self {
        Self {
            inner: guard,
            session_id,
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get the underlying lock guard
    pub fn guard(&self) -> &LockGuard {
        &self.inner
    }

    /// Check if the lock is still valid
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Release the lock explicitly
    pub fn release(self) {
        self.inner.release();
    }
}

impl Deref for SessionLockGuard {
    type Target = LockGuard;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Display for SessionLockGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionLock[{}]", self.session_id)
    }
}

/// A guard for generic resource locks
#[derive(Debug)]
pub struct ResourceLockGuard {
    inner: LockGuard,
    resource_path: String,
}

impl ResourceLockGuard {
    /// Create a new resource lock guard
    pub(crate) fn new(guard: LockGuard, resource_path: String) -> Self {
        Self {
            inner: guard,
            resource_path,
        }
    }

    /// Get the resource path
    pub fn resource_path(&self) -> &str {
        &self.resource_path
    }

    /// Get the underlying lock guard
    pub fn guard(&self) -> &LockGuard {
        &self.inner
    }

    /// Check if the lock is still valid
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Release the lock explicitly
    pub fn release(self) {
        self.inner.release();
    }
}

impl Deref for ResourceLockGuard {
    type Target = LockGuard;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Display for ResourceLockGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceLock[{}]", self.resource_path)
    }
}

/// A multi-lock guard that holds multiple locks together
///
/// All locks are released together when the guard is dropped.
/// This helps prevent deadlocks by ensuring locks are acquired and
/// released in a consistent order.
#[derive(Debug)]
pub struct MultiLockGuard {
    guards: Vec<LockGuard>,
}

impl MultiLockGuard {
    /// Create a new multi-lock guard from a list of guards
    ///
    /// The guards should be sorted by resource type priority to prevent deadlocks.
    pub(crate) fn new(guards: Vec<LockGuard>) -> Self {
        Self { guards }
    }

    /// Get the number of locks held
    pub fn len(&self) -> usize {
        self.guards.len()
    }

    /// Check if there are no locks
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    /// Check if all locks are still valid
    pub fn all_valid(&self) -> bool {
        self.guards.iter().all(|g| g.is_valid())
    }

    /// Get an iterator over the held guards
    pub fn iter(&self) -> impl Iterator<Item = &LockGuard> {
        self.guards.iter()
    }

    /// Release all locks explicitly
    pub fn release(self) {
        // Guards are released in drop order (LIFO)
        drop(self);
    }
}

impl fmt::Display for MultiLockGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MultiLock[")?;
        for (i, guard) in self.guards.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", guard)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locking::types::LockInfo;
    use std::time::Duration;

    fn create_test_lock_info(resource_type: ResourceType, resource_id: &str) -> LockInfo {
        LockInfo::new(
            resource_type,
            resource_id.to_string(),
            "test".to_string(),
            Some(Duration::from_secs(60)),
        )
    }

    #[test]
    fn test_lock_guard_validity() {
        let info = create_test_lock_info(ResourceType::Project, "test-project");
        let guard = LockGuard::new_test(info);

        assert!(guard.is_valid());
        assert_eq!(guard.resource_type(), ResourceType::Project);
        assert_eq!(guard.resource_id(), "test-project");
    }

    #[test]
    fn test_project_lock_guard() {
        let info = create_test_lock_info(ResourceType::Project, "test-project");
        let project_id = Uuid::new_v4();
        let guard = ProjectLockGuard::new(LockGuard::new_test(info), project_id);

        assert_eq!(guard.project_id(), project_id);
        assert!(guard.is_valid());
    }

    #[test]
    fn test_session_lock_guard() {
        let info = create_test_lock_info(ResourceType::Session, "test-session");
        let session_id = Uuid::new_v4();
        let guard = SessionLockGuard::new(LockGuard::new_test(info), session_id);

        assert_eq!(guard.session_id(), session_id);
        assert!(guard.is_valid());
    }

    #[test]
    fn test_resource_lock_guard() {
        let info = create_test_lock_info(ResourceType::File, "src/main.rs");
        let guard = ResourceLockGuard::new(LockGuard::new_test(info), "src/main.rs".to_string());

        assert_eq!(guard.resource_path(), "src/main.rs");
        assert!(guard.is_valid());
    }

    #[test]
    fn test_multi_lock_guard() {
        let guards = vec![
            LockGuard::new_test(create_test_lock_info(ResourceType::Project, "proj1")),
            LockGuard::new_test(create_test_lock_info(ResourceType::Session, "sess1")),
        ];

        let multi = MultiLockGuard::new(guards);

        assert_eq!(multi.len(), 2);
        assert!(!multi.is_empty());
        assert!(multi.all_valid());
    }

    #[test]
    fn test_lock_guard_display() {
        let info = create_test_lock_info(ResourceType::Project, "test-project");
        let guard = LockGuard::new_test(info);

        let display = format!("{}", guard);
        assert!(display.contains("project"));
        assert!(display.contains("test-project"));
    }
}
