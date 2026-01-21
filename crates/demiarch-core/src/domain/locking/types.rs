//! Lock types and error definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Result type for lock operations
pub type LockResult<T> = std::result::Result<T, LockError>;

/// Lock errors
#[derive(Error, Debug, Clone)]
pub enum LockError {
    /// Lock acquisition timed out
    #[error("Lock timeout: resource '{resource}' is held by {holder}")]
    Timeout {
        resource: String,
        holder: String,
    },

    /// Lock is already held by another process
    #[error("Lock contention: resource '{resource}' is held by process {holder_pid}")]
    Contention {
        resource: String,
        holder_pid: u32,
    },

    /// Lock was not found (for release operations)
    #[error("Lock not found: {0}")]
    NotFound(String),

    /// Lock is stale (holder process died)
    #[error("Stale lock detected: resource '{resource}' (holder pid {holder_pid} is not running)")]
    StaleLock {
        resource: String,
        holder_pid: u32,
    },

    /// Deadlock would occur
    #[error("Deadlock detected: acquiring {requested} while holding {held}")]
    DeadlockDetected {
        requested: String,
        held: String,
    },

    /// Invalid lock state
    #[error("Invalid lock state: {0}")]
    InvalidState(String),

    /// I/O error during lock operations
    #[error("Lock I/O error: {0}")]
    IoError(String),

    /// Lock file corruption
    #[error("Lock file corrupted: {0}")]
    Corrupted(String),
}

impl LockError {
    /// Get error code for this lock error
    pub fn code(&self) -> &'static str {
        match self {
            Self::Timeout { .. } => "E300",
            Self::Contention { .. } => "E301",
            Self::NotFound(_) => "E302",
            Self::StaleLock { .. } => "E303",
            Self::DeadlockDetected { .. } => "E304",
            Self::InvalidState(_) => "E305",
            Self::IoError(_) => "E306",
            Self::Corrupted(_) => "E307",
        }
    }
}

/// Type of resource being locked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Global workspace lock (highest priority)
    Workspace,
    /// Project-level lock
    Project,
    /// Session-level lock
    Session,
    /// Feature-level lock
    Feature,
    /// Database lock
    Database,
    /// File lock
    File,
    /// Config lock
    Config,
}

impl ResourceType {
    /// Get the lock priority (lower = higher priority, acquired first)
    pub fn priority(&self) -> u8 {
        match self {
            Self::Workspace => 0,
            Self::Database => 1,
            Self::Config => 2,
            Self::Project => 3,
            Self::Session => 4,
            Self::Feature => 5,
            Self::File => 6,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Project => "project",
            Self::Session => "session",
            Self::Feature => "feature",
            Self::Database => "database",
            Self::File => "file",
            Self::Config => "config",
        }
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Lock status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    /// Lock is available
    Available,
    /// Lock is held by this process
    HeldBySelf,
    /// Lock is held by another process
    HeldByOther,
    /// Lock is stale (holder is dead)
    Stale,
}

impl fmt::Display for LockStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::HeldBySelf => write!(f, "held_by_self"),
            Self::HeldByOther => write!(f, "held_by_other"),
            Self::Stale => write!(f, "stale"),
        }
    }
}

/// Information about a lock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Lock ID
    pub id: Uuid,

    /// Type of resource locked
    pub resource_type: ResourceType,

    /// Resource identifier (e.g., project ID, file path)
    pub resource_id: String,

    /// Current lock status
    pub status: LockStatus,

    /// Process ID of lock holder
    pub holder_pid: u32,

    /// Hostname of lock holder
    pub holder_host: String,

    /// Description of the lock holder (e.g., "agent:coder", "session:abc123")
    pub holder_description: String,

    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,

    /// When the lock expires (None = no expiry)
    pub expires_at: Option<DateTime<Utc>>,

    /// Number of times this lock has been renewed
    pub renewal_count: u32,
}

impl LockInfo {
    /// Create a new lock info for the current process
    pub fn new(
        resource_type: ResourceType,
        resource_id: String,
        holder_description: String,
        ttl: Option<Duration>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            resource_type,
            resource_id,
            status: LockStatus::HeldBySelf,
            holder_pid: std::process::id(),
            holder_host: gethostname::gethostname()
                .to_string_lossy()
                .into_owned(),
            holder_description,
            acquired_at: now,
            expires_at: ttl.map(|d| now + chrono::Duration::from_std(d).unwrap_or_default()),
            renewal_count: 0,
        }
    }

    /// Check if the lock is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Check if the lock is held by the current process
    pub fn is_held_by_self(&self) -> bool {
        self.holder_pid == std::process::id()
    }

    /// Renew the lock with a new TTL
    pub fn renew(&mut self, ttl: Duration) {
        self.expires_at = Some(Utc::now() + chrono::Duration::from_std(ttl).unwrap_or_default());
        self.renewal_count += 1;
    }

    /// Get the lock key for file-based locking
    pub fn lock_key(&self) -> String {
        format!("{}:{}", self.resource_type.as_str(), self.resource_id)
    }
}

/// Configuration for the lock manager
#[derive(Debug, Clone)]
pub struct LockConfig {
    /// Base directory for lock files
    pub lock_dir: std::path::PathBuf,

    /// Default timeout for lock acquisition
    pub default_timeout: Duration,

    /// Default TTL for locks (how long before they expire)
    pub default_ttl: Duration,

    /// Interval for checking stale locks
    pub stale_check_interval: Duration,

    /// How long a lock can be held before considered potentially stale
    pub stale_threshold: Duration,

    /// Whether to automatically clean up stale locks
    pub auto_cleanup_stale: bool,

    /// Retry interval when waiting for a lock
    pub retry_interval: Duration,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            lock_dir: std::path::PathBuf::from(".demiarch/locks"),
            default_timeout: Duration::from_secs(30),
            default_ttl: Duration::from_secs(300), // 5 minutes
            stale_check_interval: Duration::from_secs(60),
            stale_threshold: Duration::from_secs(600), // 10 minutes
            auto_cleanup_stale: true,
            retry_interval: Duration::from_millis(100),
        }
    }
}

impl LockConfig {
    /// Create a config with a custom lock directory
    pub fn with_lock_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.lock_dir = dir.into();
        self
    }

    /// Set the default timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Set the default TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_priority() {
        // Workspace should have highest priority (lowest number)
        assert!(ResourceType::Workspace.priority() < ResourceType::Project.priority());
        assert!(ResourceType::Project.priority() < ResourceType::Session.priority());
        assert!(ResourceType::Session.priority() < ResourceType::Feature.priority());
    }

    #[test]
    fn test_lock_info_creation() {
        let info = LockInfo::new(
            ResourceType::Project,
            "test-project".to_string(),
            "test-holder".to_string(),
            Some(Duration::from_secs(60)),
        );

        assert_eq!(info.resource_type, ResourceType::Project);
        assert_eq!(info.resource_id, "test-project");
        assert_eq!(info.holder_pid, std::process::id());
        assert!(info.expires_at.is_some());
        assert!(!info.is_expired());
    }

    #[test]
    fn test_lock_info_renewal() {
        let mut info = LockInfo::new(
            ResourceType::Session,
            "test-session".to_string(),
            "test-holder".to_string(),
            Some(Duration::from_secs(1)),
        );

        assert_eq!(info.renewal_count, 0);

        info.renew(Duration::from_secs(60));

        assert_eq!(info.renewal_count, 1);
        assert!(!info.is_expired());
    }

    #[test]
    fn test_lock_key() {
        let info = LockInfo::new(
            ResourceType::Project,
            "abc123".to_string(),
            "test".to_string(),
            None,
        );

        assert_eq!(info.lock_key(), "project:abc123");
    }

    #[test]
    fn test_lock_status_display() {
        assert_eq!(LockStatus::Available.to_string(), "available");
        assert_eq!(LockStatus::HeldBySelf.to_string(), "held_by_self");
        assert_eq!(LockStatus::HeldByOther.to_string(), "held_by_other");
        assert_eq!(LockStatus::Stale.to_string(), "stale");
    }

    #[test]
    fn test_lock_error_codes() {
        let timeout_err = LockError::Timeout {
            resource: "test".to_string(),
            holder: "other".to_string(),
        };
        assert_eq!(timeout_err.code(), "E300");

        let contention_err = LockError::Contention {
            resource: "test".to_string(),
            holder_pid: 1234,
        };
        assert_eq!(contention_err.code(), "E301");
    }

    #[test]
    fn test_lock_config_builder() {
        let config = LockConfig::default()
            .with_lock_dir("/tmp/locks")
            .with_timeout(Duration::from_secs(60))
            .with_ttl(Duration::from_secs(120));

        assert_eq!(config.lock_dir, std::path::PathBuf::from("/tmp/locks"));
        assert_eq!(config.default_timeout, Duration::from_secs(60));
        assert_eq!(config.default_ttl, Duration::from_secs(120));
    }
}
