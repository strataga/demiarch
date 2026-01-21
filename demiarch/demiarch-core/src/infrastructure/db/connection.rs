//! Database connection management utilities

use anyhow::{anyhow, Context};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

/// Create a SQLite connection options from a database path
pub fn create_connection_options(path: &Path) -> anyhow::Result<SqliteConnectOptions> {
    validate_database_path(path)?;

    let database_url = format!("sqlite:{}", path.display());

    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(30));

    Ok(options)
}

pub fn ensure_secure_permissions(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if path.exists() {
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }
    }

    Ok(())
}

/// Get database file size in bytes
pub async fn get_database_size(path: &Path) -> anyhow::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let metadata = tokio::fs::metadata(path).await?;
    Ok(metadata.len())
}

/// Check if database file exists
pub fn database_exists(path: &Path) -> bool {
    path.exists()
}

/// Create database directory structure
pub async fn ensure_database_directory(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        // Avoid writing into symlinked parents
        deny_parent_symlink(parent)?;

        tokio::fs::create_dir_all(parent).await?;
        harden_directory_permissions(parent).await?;

        // Re-check after creation in case the directory was replaced concurrently
        deny_parent_symlink(parent)?;
    }
    Ok(())
}

async fn harden_directory_permissions(parent: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if parent.exists() {
            let mut perms = tokio::fs::metadata(parent).await?.permissions();
            perms.set_mode(0o700);
            tokio::fs::set_permissions(parent, perms).await?;
        }
    }

    Ok(())
}

pub(crate) fn deny_parent_symlink(parent: &Path) -> anyhow::Result<()> {
    if parent.exists() {
        let meta = std::fs::symlink_metadata(parent)?;
        if meta.file_type().is_symlink() {
            return Err(anyhow!("Database parent directory cannot be a symlink"));
        }
    }
    Ok(())
}

pub(crate) fn validate_database_path(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        deny_parent_symlink(parent)?;
    }

    if path.exists() {
        let meta = std::fs::symlink_metadata(path)
            .with_context(|| format!("Failed to read metadata for {:?}", path))?;

        if meta.file_type().is_symlink() {
            return Err(anyhow!("Database path cannot be a symlink"));
        }

        if !meta.file_type().is_file() {
            return Err(anyhow!("Database path must be a regular file"));
        }
    }

    Ok(())
}

/// Database connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Total connections in pool
    pub total_connections: u32,
    /// Idle connections
    pub idle_connections: u32,
    /// Active connections
    pub active_connections: u32,
}

/// Get connection pool statistics
pub fn get_connection_stats(pool: &SqlitePool) -> ConnectionStats {
    let total = pool.size();
    let idle = pool.num_idle() as u32;
    let active = total.saturating_sub(idle);

    ConnectionStats {
        total_connections: total,
        idle_connections: idle,
        active_connections: active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_connection_options() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = create_connection_options(&db_path).unwrap();

        // Ensure WAL mode and foreign keys are configured
        let options_str = format!("{:?}", options);
        assert!(options_str.contains("Wal") || options_str.contains("WAL"));
    }

    #[test]
    fn test_database_exists() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path().join("existing.db");
        let non_existent_path = temp_dir.path().join("nonexistent.db");

        // Create the existing file
        std::fs::File::create(&existing_path).unwrap();

        assert!(database_exists(&existing_path));
        assert!(!database_exists(&non_existent_path));
    }

    #[tokio::test]
    async fn test_ensure_database_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("nested").join("dir").join("test.db");

        let result = ensure_database_directory(&db_path).await;
        assert!(result.is_ok());

        // Verify directory was created
        let parent = db_path.parent().unwrap();
        assert!(parent.exists());
    }

    #[tokio::test]
    async fn test_get_database_size() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Non-existent file should return 0
        let size = get_database_size(&db_path).await.unwrap();
        assert_eq!(size, 0);

        // Create file with some content
        std::fs::write(&db_path, b"test data").unwrap();
        let size = get_database_size(&db_path).await.unwrap();
        assert_eq!(size, 9);
    }

    #[test]
    fn test_connection_stats() {
        // Test struct fields
        let stats = ConnectionStats {
            total_connections: 10,
            idle_connections: 3,
            active_connections: 7,
        };

        assert_eq!(stats.total_connections, 10);
        assert_eq!(stats.idle_connections, 3);
        assert_eq!(stats.active_connections, 7);
    }
}
