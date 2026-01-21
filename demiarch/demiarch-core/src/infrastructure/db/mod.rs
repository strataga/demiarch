//! Database infrastructure for Demiarch
//!
//! Provides SQLite database connectivity, connection pooling, migrations,
//! and database utilities for per-project data storage.

pub mod connection;
pub mod migrations;
pub mod schema;
pub mod types;
pub mod utils;

pub use connection::*;
pub use migrations::*;
pub use schema::*;
pub use types::*;
pub use utils::*;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::{Path, PathBuf};
use tracing::info;

/// Database configuration for a project
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the database file
    pub path: PathBuf,
    /// Maximum connections in the pool
    pub max_connections: u32,
    /// Minimum connections to keep in the pool
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("database.sqlite"),
            max_connections: 10,
            min_connections: 2,
            connect_timeout: 30,
        }
    }
}

/// Database manager for project-specific databases
#[derive(Debug)]
pub struct DatabaseManager {
    config: DatabaseConfig,
    pool: Option<SqlitePool>,
}

impl DatabaseManager {
    /// Create a new database manager with default configuration
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config, pool: None }
    }

    /// Initialize the database connection pool and run migrations
    ///
    /// This method creates the connection pool and automatically runs
    /// any pending database migrations to ensure the schema is up to date.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Ok if initialization was successful, Err otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database connection cannot be established
    /// - Migration execution fails
    /// - Database schema is invalid
    pub async fn initialize(&mut self) -> Result<()> {
        self.pool = Some(self.create_pool().await?);

        // Run migrations automatically
        self.run_migrations().await?;

        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> Result<&SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized. Call initialize() first."))
    }

    /// Create a new connection pool
    async fn create_pool(&self) -> Result<SqlitePool> {
        crate::infrastructure::db::connection::validate_database_path(&self.config.path)?;

        crate::infrastructure::db::connection::ensure_database_directory(&self.config.path).await?;

        let options =
            crate::infrastructure::db::connection::create_connection_options(&self.config.path)?;

        let pool = SqlitePoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(self.config.connect_timeout))
            .connect_with(options)
            .await?;

        crate::infrastructure::db::connection::ensure_secure_permissions(&self.config.path)?;

        Ok(pool)
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        let pool = self.pool()?;
        migrations::run_migrations(pool).await
    }

    /// Create database backup
    pub async fn create_backup(&self, backup_path: &Path) -> Result<()> {
        let pool = self.pool()?;
        let target = self.normalize_backup_path(backup_path).await?;

        // Use SQLite backup API with parameterized path
        let mut conn = pool.begin().await?;

        sqlx::query("VACUUM INTO ?")
            .bind(target.to_string_lossy().to_string())
            .execute(&mut *conn)
            .await?;

        conn.commit().await?;

        // Harden permissions (0600 file, 0700 dir)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(parent) = target.parent() {
                let mut perms = tokio::fs::metadata(parent).await?.permissions();
                perms.set_mode(0o700);
                tokio::fs::set_permissions(parent, perms).await?;
            }

            let mut perms = tokio::fs::metadata(&target).await?.permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&target, perms).await?;
        }

        Ok(())
    }

    /// Restore database from backup
    pub async fn restore_from_backup(&mut self, backup_path: &Path) -> Result<()> {
        let validated_backup = self
            .validate_backup_source(backup_path)
            .await
            .map_err(|e| anyhow::anyhow!("Backup file validation failed: {e}"))?;

        // Close existing connections
        if let Some(pool) = &self.pool {
            pool.close().await;
        }

        // Copy backup file to database location
        tokio::fs::copy(&validated_backup, &self.config.path).await?;

        // Reset permissions to 0600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&self.config.path).await?.permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&self.config.path, perms).await?;
        }

        // Reinitialize the pool
        self.initialize().await?;

        // Run migrations to ensure schema is up to date
        self.run_migrations().await?;

        Ok(())
    }

    async fn validate_backup_source(&self, backup_path: &Path) -> Result<PathBuf> {
        if !backup_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup file does not exist: {:?}",
                backup_path
            ));
        }

        let metadata = tokio::fs::symlink_metadata(backup_path).await?;
        if metadata.file_type().is_symlink() {
            return Err(anyhow::anyhow!("Backup path cannot be a symlink"));
        }
        if !metadata.file_type().is_file() {
            return Err(anyhow::anyhow!("Backup path must be a regular file"));
        }

        let canonical_backup = tokio::fs::canonicalize(backup_path).await?;

        ensure_not_symlink(&canonical_backup)?;

        let base_dir = self
            .config
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");

        let canonical_base = tokio::fs::canonicalize(&base_dir)
            .await
            .unwrap_or(base_dir.clone());

        ensure_not_symlink(&canonical_base)?;

        if !canonical_backup.starts_with(&canonical_base) {
            return Err(anyhow::anyhow!(
                "Backup must reside under the project's backups directory: {:?}",
                canonical_base
            ));
        }

        let ext = canonical_backup
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        const ALLOWED_EXTENSIONS: &[&str] = &["sqlite", "db", "bak"];
        if !ALLOWED_EXTENSIONS.contains(&ext) {
            return Err(anyhow::anyhow!(
                "Backup file must use one of the allowed extensions: {:?}",
                ALLOWED_EXTENSIONS
            ));
        }

        Ok(canonical_backup)
    }

    async fn normalize_backup_path(&self, backup_path: &Path) -> Result<PathBuf> {
        use std::path::{Path, PathBuf as StdPathBuf};

        let base_dir: StdPathBuf = self
            .config
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");

        ensure_not_symlink(&base_dir)?;
        tokio::fs::create_dir_all(&base_dir).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut dir_perms = tokio::fs::metadata(&base_dir).await?.permissions();
            dir_perms.set_mode(0o700);
            tokio::fs::set_permissions(&base_dir, dir_perms).await?;
        }

        let file_name = backup_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Backup file name is required"))?;

        let candidate = base_dir.join(file_name);

        const ALLOWED_EXTENSIONS: &[&str] = &["sqlite", "db", "bak"];
        let ext = candidate.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ALLOWED_EXTENSIONS.contains(&ext) {
            return Err(anyhow::anyhow!(
                "Backup file must use one of the allowed extensions: {:?}",
                ALLOWED_EXTENSIONS
            ));
        }

        if candidate.to_string_lossy().contains('\'') {
            return Err(anyhow::anyhow!("Backup path contains invalid characters"));
        }

        Ok(candidate)
    }

    /// Get database health status
    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let pool = self.pool()?;

        let mut conn = pool.begin().await?;
        let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(&mut *conn)
            .await;

        let is_healthy = result.is_ok();
        let table_count = if is_healthy {
            result.unwrap().len() as u32
        } else {
            0
        };

        Ok(DatabaseHealth {
            is_healthy,
            table_count,
            connection_count: pool.size(),
            idle_connection_count: pool.num_idle() as u32,
        })
    }
}

fn ensure_not_symlink(path: &Path) -> Result<()> {
    if path.exists() {
        let meta = std::fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(anyhow::anyhow!(
                "Symlinks are not allowed for database paths"
            ));
        }
    }

    Ok(())
}

/// Database health status
#[derive(Debug, Clone)]
pub struct DatabaseHealth {
    /// Whether the database is healthy
    pub is_healthy: bool,
    /// Number of tables in the database
    pub table_count: u32,
    /// Number of active connections
    pub connection_count: u32,
    /// Number of idle connections
    pub idle_connection_count: u32,
}

/// Create a database config for a project
pub fn create_project_config(project_id: &str) -> anyhow::Result<DatabaseConfig> {
    crate::infrastructure::db::utils::DatabaseUtils::validate_project_id(project_id)
        .map_err(|e| anyhow::anyhow!("Invalid project id: {}", e))?;

    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    let base_dir = home_dir.join(".demiarch").join("projects");
    let project_dir = base_dir.join(project_id);
    let path = project_dir.join("database.sqlite");

    if !path.starts_with(&base_dir) {
        return Err(anyhow::anyhow!("Database path escaped base directory"));
    }

    Ok(DatabaseConfig {
        path,
        max_connections: 10,
        min_connections: 2,
        connect_timeout: 30,
    })
}

/// Create a new project database with proper permissions
///
/// This function creates the database file and sets secure permissions (0600)
/// to ensure only the current user can read/write the database file.
///
/// # Arguments
///
/// * `project_id` - The unique identifier for the project
///
/// # Returns
///
/// * `Result<()>` - Ok if database was created successfully, Err otherwise
///
/// # Examples
///
/// ```rust
/// use demiarch_core::infrastructure::db::create_project_database;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     create_project_database("my-project").await?;
///     Ok(())
/// }
/// ```
pub async fn create_project_database(project_id: &str) -> Result<()> {
    let config = create_project_config(project_id)?;
    let db_path = &config.path;

    crate::infrastructure::db::connection::validate_database_path(db_path)?;

    crate::infrastructure::db::connection::ensure_database_directory(db_path).await?;

    // Create the database file by connecting to it (SQLite creates the file on first connection)
    let options = crate::infrastructure::db::connection::create_connection_options(db_path)?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect_with(options)
        .await?;

    // Set file permissions to 0600 (user read/write only)
    crate::infrastructure::db::connection::ensure_secure_permissions(db_path)?;

    // Close pool
    pool.close().await;

    info!("Created project database: {}", db_path.display());

    Ok(())
}

/// Initialize a project database with migrations
///
/// This function creates the database file (if it doesn't exist),
/// sets secure permissions, and runs all pending migrations.
///
/// # Arguments
///
/// * `project_id` - The unique identifier for the project
///
/// # Returns
///
/// * `Result<DatabaseManager>` - Configured DatabaseManager ready for use
pub async fn initialize_project_database(project_id: &str) -> Result<DatabaseManager> {
    // Create database file with proper permissions
    create_project_database(project_id).await?;

    // Create database manager and initialize
    let config = create_project_config(project_id)?;
    let mut manager = DatabaseManager::new(config);
    manager.initialize().await?;

    // Run migrations
    manager.run_migrations().await?;

    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_database_config() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.db");
        let config = DatabaseConfig {
            path: path.clone(),
            ..Default::default()
        };

        assert_eq!(config.path, path);
    }

    #[test]
    fn test_database_health_struct() {
        let health = DatabaseHealth {
            is_healthy: true,
            table_count: 10,
            connection_count: 5,
            idle_connection_count: 2,
        };

        assert!(health.is_healthy);
        assert_eq!(health.table_count, 10);
        assert_eq!(health.connection_count, 5);
        assert_eq!(health.idle_connection_count, 2);
    }
}
