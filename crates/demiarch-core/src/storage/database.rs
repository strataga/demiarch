//! SQLite database operations
//!
//! Provides connection pool management and database initialization for demiarch.

use crate::storage::migrations;
use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Default maximum connections in the pool
const DEFAULT_MAX_CONNECTIONS: u32 = 5;

/// Database configuration options
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub path: PathBuf,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Whether to run migrations automatically
    pub auto_migrate: bool,
    /// Journal mode (default: WAL for better concurrency)
    pub journal_mode: SqliteJournalMode,
    /// Synchronous mode (default: NORMAL for balance of safety/performance)
    pub synchronous: SqliteSynchronous,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_database_path(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            auto_migrate: true,
            journal_mode: SqliteJournalMode::Wal,
            synchronous: SqliteSynchronous::Normal,
        }
    }
}

impl DatabaseConfig {
    /// Create a new database config with the specified path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    /// Create a config for an in-memory database (useful for testing)
    pub fn in_memory() -> Self {
        Self {
            path: PathBuf::from(":memory:"),
            max_connections: 1, // In-memory requires single connection
            auto_migrate: true,
            journal_mode: SqliteJournalMode::Wal,
            synchronous: SqliteSynchronous::Normal,
        }
    }

    /// Set the maximum number of connections
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Disable automatic migrations
    pub fn no_migrate(mut self) -> Self {
        self.auto_migrate = false;
        self
    }
}

/// Get the default database path
pub fn default_database_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("demiarch").join("demiarch.db")
    } else {
        PathBuf::from("demiarch.db")
    }
}

/// Get the database path for a specific project
pub fn project_database_path(project_dir: &Path) -> PathBuf {
    project_dir.join(".demiarch").join("project.db")
}

/// Database connection pool wrapper
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    config: DatabaseConfig,
}

impl Database {
    /// Create a new database connection with the given configuration
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        // Ensure the directory exists
        if let Some(parent) = config.path.parent() {
            if !parent.exists() && config.path.to_string_lossy() != ":memory:" {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
            }
        }

        let connection_str = if config.path.to_string_lossy() == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{}?mode=rwc", config.path.display())
        };

        let connect_options = SqliteConnectOptions::from_str(&connection_str)?
            .journal_mode(config.journal_mode)
            .synchronous(config.synchronous)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect_with(connect_options)
            .await
            .with_context(|| format!("Failed to connect to database: {:?}", config.path))?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        let db = Self {
            pool,
            config: config.clone(),
        };

        // Run migrations if auto_migrate is enabled
        if config.auto_migrate {
            db.migrate().await?;
        }

        Ok(db)
    }

    /// Create a database connection with default configuration
    pub async fn default() -> Result<Self> {
        Self::new(DatabaseConfig::default()).await
    }

    /// Create an in-memory database (useful for testing)
    pub async fn in_memory() -> Result<Self> {
        Self::new(DatabaseConfig::in_memory()).await
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get the database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        migrations::run_migrations(&self.pool)
            .await
            .context("Failed to run database migrations")
    }

    /// Check migration status
    pub async fn migration_status(&self) -> Result<migrations::MigrationStatus> {
        migrations::migration_status(&self.pool)
            .await
            .context("Failed to check migration status")
    }

    /// Check if database is healthy
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    /// Close the database connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Get the database file path
    pub fn path(&self) -> &Path {
        &self.config.path
    }
}

/// Application-wide database manager
///
/// Manages both the global database and project-specific databases.
#[derive(Debug, Clone)]
pub struct DatabaseManager {
    /// Global database for application-wide data (e.g., encrypted keys, global settings)
    global: Database,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new() -> Result<Self> {
        let global = Database::default().await?;
        Ok(Self { global })
    }

    /// Create a database manager with custom global database path
    pub async fn with_global_path(path: impl Into<PathBuf>) -> Result<Self> {
        let global = Database::new(DatabaseConfig::with_path(path)).await?;
        Ok(Self { global })
    }

    /// Create an in-memory database manager (useful for testing)
    pub async fn in_memory() -> Result<Self> {
        let global = Database::in_memory().await?;
        Ok(Self { global })
    }

    /// Get the global database
    pub fn global(&self) -> &Database {
        &self.global
    }

    /// Open a project-specific database
    pub async fn open_project(&self, project_dir: &Path) -> Result<Database> {
        let path = project_database_path(project_dir);
        Database::new(DatabaseConfig::with_path(path)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database() {
        let db = Database::in_memory().await.expect("Failed to create in-memory database");

        // Health check should pass
        db.health_check().await.expect("Health check failed");

        // Migrations should have run
        let status = db.migration_status().await.expect("Failed to get migration status");
        assert!(!status.needs_migration);
    }

    #[tokio::test]
    async fn test_database_config_builder() {
        let config = DatabaseConfig::with_path("/tmp/test.db")
            .max_connections(10)
            .no_migrate();

        assert_eq!(config.path, PathBuf::from("/tmp/test.db"));
        assert_eq!(config.max_connections, 10);
        assert!(!config.auto_migrate);
    }

    #[tokio::test]
    async fn test_database_manager_in_memory() {
        let manager = DatabaseManager::in_memory()
            .await
            .expect("Failed to create database manager");

        // Global database should be accessible
        manager
            .global()
            .health_check()
            .await
            .expect("Global database health check failed");
    }

    #[tokio::test]
    async fn test_foreign_keys_enabled() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Check that foreign keys are enabled
        let result: (i32,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(db.pool())
            .await
            .expect("Failed to check foreign_keys pragma");

        assert_eq!(result.0, 1, "Foreign keys should be enabled");
    }

    #[tokio::test]
    async fn test_database_crud_operations() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Insert a project
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO projects (id, name, framework) VALUES (?, ?, ?)")
            .bind(&project_id)
            .bind("Test Project")
            .bind("rust")
            .execute(db.pool())
            .await
            .expect("Failed to insert project");

        // Query it back
        let (name,): (String,) = sqlx::query_as("SELECT name FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query project");

        assert_eq!(name, "Test Project");

        // Update it
        sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
            .bind("Updated Project")
            .bind(&project_id)
            .execute(db.pool())
            .await
            .expect("Failed to update project");

        // Verify update
        let (name,): (String,) = sqlx::query_as("SELECT name FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query updated project");

        assert_eq!(name, "Updated Project");

        // Delete it
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(&project_id)
            .execute(db.pool())
            .await
            .expect("Failed to delete project");

        // Verify deletion
        let result: Option<(String,)> = sqlx::query_as("SELECT name FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_optional(db.pool())
            .await
            .expect("Failed to query deleted project");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Insert a project
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind(&project_id)
            .bind("Test Project")
            .execute(db.pool())
            .await
            .expect("Failed to insert project");

        // Insert a feature for the project
        let feature_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO features (id, project_id, title) VALUES (?, ?, ?)")
            .bind(&feature_id)
            .bind(&project_id)
            .bind("Test Feature")
            .execute(db.pool())
            .await
            .expect("Failed to insert feature");

        // Verify feature exists
        let result: Option<(String,)> = sqlx::query_as("SELECT title FROM features WHERE id = ?")
            .bind(&feature_id)
            .fetch_optional(db.pool())
            .await
            .expect("Failed to query feature");
        assert!(result.is_some());

        // Delete the project (should cascade to features)
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(&project_id)
            .execute(db.pool())
            .await
            .expect("Failed to delete project");

        // Verify feature was deleted via cascade
        let result: Option<(String,)> = sqlx::query_as("SELECT title FROM features WHERE id = ?")
            .bind(&feature_id)
            .fetch_optional(db.pool())
            .await
            .expect("Failed to query deleted feature");
        assert!(result.is_none(), "Feature should be deleted via cascade");
    }
}
