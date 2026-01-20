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
use sqlx::SqlitePool;
use std::path::PathBuf;
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
        let database_url = format!("sqlite:{}", self.config.path.display());

        // Ensure database directory exists
        if let Some(parent) = self.config.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let pool = sqlx::SqlitePool::connect(&database_url).await?;

        Ok(pool)
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        let pool = self.pool()?;
        migrations::run_migrations(pool).await
    }

    /// Create database backup
    pub async fn create_backup(&self, backup_path: &PathBuf) -> Result<()> {
        let pool = self.pool()?;

        // Create backup directory if it doesn't exist
        if let Some(parent) = backup_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Use SQLite backup API
        let mut conn = pool.begin().await?;
        let backup_path_str = backup_path.to_string_lossy();

        sqlx::query(&format!("VACUUM INTO '{}'", backup_path_str))
            .execute(&mut *conn)
            .await?;

        conn.commit().await?;
        Ok(())
    }

    /// Restore database from backup
    pub async fn restore_from_backup(&mut self, backup_path: &PathBuf) -> Result<()> {
        if !backup_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup file does not exist: {:?}",
                backup_path
            ));
        }

        // Close existing connections
        if let Some(pool) = &self.pool {
            pool.close().await;
        }

        // Copy backup file to database location
        tokio::fs::copy(backup_path, &self.config.path).await?;

        // Reinitialize the pool
        self.initialize().await?;

        // Run migrations to ensure schema is up to date
        self.run_migrations().await?;

        Ok(())
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
            connection_count: pool.size() as u32,
            idle_connection_count: pool.num_idle() as u32,
        })
    }
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
pub fn create_project_config(project_id: &str) -> DatabaseConfig {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let project_dir = home_dir.join(".demiarch").join("projects").join(project_id);

    DatabaseConfig {
        path: project_dir.join("database.sqlite"),
        max_connections: 10,
        min_connections: 2,
        connect_timeout: 30,
    }
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
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let config = create_project_config(project_id);
    let db_path = &config.path;

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Create the database file by connecting to it (SQLite creates the file on first connection)
    let database_url = format!("sqlite:{}", db_path.display());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await?;

    // Set file permissions to 0600 (user read/write only)
    let mut perms = fs::metadata(db_path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(db_path, perms)?;

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
    let config = create_project_config(project_id);
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
