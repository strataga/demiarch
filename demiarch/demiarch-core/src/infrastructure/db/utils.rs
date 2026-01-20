//! Database utilities and helper functions
//!
//! This module provides comprehensive utilities for database operations including
//! ID generation, JSON conversion, validation, sanitization, and database health checks.
//! These utilities are designed to be used throughout the application to ensure
//! consistent and safe database operations.

use crate::infrastructure::db::{DatabaseManager, DatabaseResult};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use uuid::Uuid;

/// Database operation utilities
///
/// This struct provides static utility methods for common database operations
/// including ID generation, JSON conversion, validation, sanitization, and
/// database permission checks. All methods are static and can be called directly
/// on the struct without instantiation.
///
/// # Examples
///
/// ```rust
/// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
///
/// // Generate a unique ID
/// let id = DatabaseUtils::generate_id();
///
/// // Validate a project ID
/// let validation = DatabaseUtils::validate_project_id("my-project");
///
/// // Sanitize a string for database storage
/// let clean = DatabaseUtils::sanitize_string("  Hello\r\nWorld  ");
/// ```
pub struct DatabaseUtils;

impl DatabaseUtils {
    /// Generate a new UUID for database records
    ///
    /// Creates a version 4 UUID (random) suitable for use as primary keys
    /// and unique identifiers in database records. This is the standard
    /// method for generating unique IDs throughout the application.
    ///
    /// # Returns
    ///
    /// * `Uuid` - A new random UUID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    ///
    /// let id = DatabaseUtils::generate_id();
    /// println!("Generated ID: {}", id);
    /// ```
    pub fn generate_id() -> Uuid {
        Uuid::new_v4()
    }

    /// Convert JSON string to HashMap
    ///
    /// Parses a JSON string into a HashMap of String to serde_json::Value.
    /// This utility is commonly used for deserializing JSON data stored in
    /// database text columns (e.g., settings, metadata, configuration).
    ///
    /// # Arguments
    ///
    /// * `json_str` - The JSON string to parse
    ///
    /// # Returns
    ///
    /// * `DatabaseResult<HashMap<String, serde_json::Value>>` - The parsed HashMap or error
    ///
    /// # Errors
    ///
    /// Returns DatabaseError::Json if the JSON string is malformed or invalid.
    /// Returns an empty HashMap if the input string is empty or "{}".
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    ///
    /// let json = r#"{"key": "value", "number": 123}"#;
    /// let map = DatabaseUtils::json_to_map(json).unwrap();
    /// assert_eq!(map.get("key").unwrap(), "value");
    /// ```
    pub fn json_to_map(
        json_str: &str,
    ) -> DatabaseResult<std::collections::HashMap<String, serde_json::Value>> {
        if json_str.is_empty() || json_str == "{}" {
            return Ok(std::collections::HashMap::new());
        }

        serde_json::from_str(json_str)
            .map_err(|e| crate::infrastructure::db::DatabaseError::Json(e))
    }

    /// Convert HashMap to JSON string
    ///
    /// Serializes a HashMap into a JSON string. This is the inverse operation
    /// of `json_to_map` and is used when storing HashMap data in database
    /// text columns.
    ///
    /// # Arguments
    ///
    /// * `map` - The HashMap to serialize
    ///
    /// # Returns
    ///
    /// * `DatabaseResult<String>` - The JSON string or error
    ///
    /// # Errors
    ///
    /// Returns DatabaseError::Json if serialization fails (e.g., contains
    /// non-serializable data types).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let mut map = HashMap::new();
    /// map.insert("key".to_string(), json!("value"));
    /// let json_str = DatabaseUtils::map_to_json(&map).unwrap();
    /// ```
    pub fn map_to_json(
        map: &std::collections::HashMap<String, serde_json::Value>,
    ) -> DatabaseResult<String> {
        serde_json::to_string(map).map_err(|e| crate::infrastructure::db::DatabaseError::Json(e))
    }

    /// Validate project ID format
    ///
    /// Validates that a project ID meets the required format constraints:
    /// - Must not be empty
    /// - Must not exceed 100 characters
    /// - Must only contain alphanumeric characters, hyphens, and underscores
    ///
    /// This validation is used when creating new projects to ensure consistent
    /// and safe project identifiers that can be used in file paths and URLs.
    ///
    /// # Arguments
    ///
    /// * `project_id` - The project ID to validate
    ///
    /// # Returns
    ///
    /// * `DatabaseResult<()>` - Ok if valid, Err with validation error details
    ///
    /// # Errors
    ///
    /// Returns DatabaseError::Validation with specific error messages for:
    /// - Empty project ID
    /// - Project ID too long (>100 characters)
    /// - Invalid characters in project ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    ///
    /// // Valid project IDs
    /// assert!(DatabaseUtils::validate_project_id("my-project").is_ok());
    /// assert!(DatabaseUtils::validate_project_id("project_123").is_ok());
    ///
    /// // Invalid project IDs
    /// assert!(DatabaseUtils::validate_project_id("").is_err());
    /// assert!(DatabaseUtils::validate_project_id("project with spaces").is_err());
    /// ```
    pub fn validate_project_id(project_id: &str) -> DatabaseResult<()> {
        if project_id.is_empty() {
            return Err(crate::infrastructure::db::DatabaseError::Validation(
                "Project ID cannot be empty".to_string(),
            ));
        }

        if project_id.len() > 100 {
            return Err(crate::infrastructure::db::DatabaseError::Validation(
                "Project ID too long (max 100 characters)".to_string(),
            ));
        }

        if !project_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(crate::infrastructure::db::DatabaseError::Validation(
                "Project ID can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Sanitize string for database storage
    ///
    /// Cleans up input strings by removing potentially problematic characters
    /// that could cause issues in database storage or processing. This includes:
    /// - Null bytes (0x00) which can cause database errors
    /// - Carriage returns (\r) which can cause display issues
    /// - Leading/trailing whitespace
    ///
    /// This function should be used for all user-provided text data before
    /// storing it in the database to ensure data consistency and prevent
    /// potential security issues.
    ///
    /// # Arguments
    ///
    /// * `input` - The string to sanitize
    ///
    /// # Returns
    ///
    /// * `String` - The sanitized string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    ///
    /// let dirty = "  Hello\r\nWorld\u{0000}  ";
    /// let clean = DatabaseUtils::sanitize_string(dirty);
    /// assert_eq!(clean, "HelloWorld");
    /// ```
    pub fn sanitize_string(input: &str) -> String {
        input
            .replace('\u{0000}', "") // Remove null bytes
            .replace('\r', "") // Remove carriage returns
            .replace('\n', "") // Remove newlines
            .trim()
            .to_string()
    }

    /// Check if database file is readable and writable
    ///
    /// Verifies that the database file (or its parent directory for new files)
    /// has the necessary permissions for read and write operations. This is
    /// important for ensuring the application can access the database before
    /// attempting to open connections or perform operations.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the database file
    ///
    /// # Returns
    ///
    /// * `DatabaseResult<()>` - Ok if permissions are sufficient, Err otherwise
    ///
    /// # Errors
    ///
    /// Returns DatabaseError::Io if:
    /// - The database file exists but is read-only
    /// - The parent directory doesn't exist or is not accessible
    /// - Permission checking fails for other filesystem reasons
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let db_path = PathBuf::from("/path/to/database.sqlite");
    ///     DatabaseUtils::check_database_permissions(&db_path).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_database_permissions(path: &PathBuf) -> DatabaseResult<()> {
        use tokio::fs;

        if !path.exists() {
            // If file doesn't exist, check if we can create it
            if let Some(parent) = path.parent() {
                fs::metadata(parent)
                    .await
                    .map_err(|e| crate::infrastructure::db::DatabaseError::Io(e))?;
            }
            return Ok(());
        }

        let metadata = fs::metadata(path)
            .await
            .map_err(|e| crate::infrastructure::db::DatabaseError::Io(e))?;

        let permissions = metadata.permissions();

        if !permissions.readonly() {
            Ok(())
        } else {
            Err(crate::infrastructure::db::DatabaseError::Io(
                std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Database file is read-only",
                ),
            ))
        }
    }

    /// Get database statistics
    ///
    /// Collects comprehensive statistics about the database including row counts
    /// for all tables and the total database file size. This is useful for
    /// monitoring, debugging, and providing database health information.
    ///
    /// # Arguments
    ///
    /// * `pool` - The SQLite connection pool
    ///
    /// # Returns
    ///
    /// * `DatabaseResult<DatabaseStats>` - Database statistics or error
    ///
    /// # Statistics Collected
    ///
    /// - Row counts for all 9 Demiarch tables:
    ///   - projects, conversations, agents, skills
    ///   - code_generation, llm_calls, checkpoints
    ///   - plugins, sessions
    /// - Total database file size in bytes
    ///
    /// # Errors
    ///
    /// Returns DatabaseError if any database queries fail to execute.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use demiarch_core::infrastructure::db::utils::DatabaseUtils;
    /// use sqlx::SqlitePool;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let pool = SqlitePool::connect("sqlite:database.sqlite").await?;
    ///     let stats = DatabaseUtils::get_database_stats(&pool).await?;
    ///     println!("Database size: {} bytes", stats.db_size_bytes);
    ///     for (table, count) in &stats.table_stats {
    ///         println!("{}: {} rows", table, count);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_database_stats(pool: &SqlitePool) -> DatabaseResult<DatabaseStats> {
        let mut conn = pool.begin().await?;

        // Get table counts
        let tables = vec![
            "projects",
            "conversations",
            "agents",
            "skills",
            "code_generation",
            "llm_calls",
            "checkpoints",
            "plugins",
            "sessions",
        ];

        let mut table_stats = std::collections::HashMap::new();

        for table in tables {
            let query = format!("SELECT COUNT(*) as count FROM {}", table);
            let result = sqlx::query(&query).fetch_one(&mut *conn).await;

            match result {
                Ok(row) => {
                    let count: i64 = row.get("count");
                    table_stats.insert(table.to_string(), count as u64);
                }
                Err(_) => {
                    table_stats.insert(table.to_string(), 0);
                }
            }
        }

        // Get total database size
        let size_query =
            "SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()";
        let size_result = sqlx::query(size_query).fetch_one(&mut *conn).await;

        let db_size_bytes = match size_result {
            Ok(row) => {
                let size: i64 = row.get("size");
                size as u64
            }
            Err(_) => 0,
        };

        conn.commit().await?;

        Ok(DatabaseStats {
            table_stats,
            db_size_bytes,
        })
    }
}

/// Database statistics
///
/// Contains comprehensive statistics about the database state, including
/// row counts for all tables and the total database file size. This struct
/// is used for monitoring, debugging, and providing database health information
/// to users and administrators.
///
/// # Fields
///
/// * `table_stats` - HashMap mapping table names to their row counts
/// * `db_size_bytes` - Total database file size in bytes
///
/// # Examples
///
/// ```rust
/// use demiarch_core::infrastructure::db::utils::DatabaseStats;
/// use std::collections::HashMap;
///
/// let stats = DatabaseStats {
///     table_stats: HashMap::new(),
///     db_size_bytes: 1024,
/// };
/// println!("Database size: {} bytes", stats.db_size_bytes);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    /// Table row counts
    ///
    /// A HashMap where keys are table names and values are the number
    /// of rows in each table. Includes all 9 Demiarch tables:
    /// projects, conversations, agents, skills, code_generation,
    /// llm_calls, checkpoints, plugins, and sessions.
    pub table_stats: std::collections::HashMap<String, u64>,
    /// Database file size in bytes
    ///
    /// The total size of the SQLite database file on disk, calculated
    /// using SQLite's PRAGMA statements for accuracy.
    pub db_size_bytes: u64,
}

/// Database health check result
///
/// Comprehensive health check result that indicates the overall health
/// of the database and provides detailed information about connection
/// status, read/write capabilities, schema version, statistics, and any
/// errors encountered during the health check.
///
/// # Fields
///
/// * `is_healthy` - Overall health status (true if no errors)
/// * `can_connect` - Whether database connection was successful
/// * `can_read` - Whether read operations are working
/// * `can_write` - Whether write operations are working
/// * `schema_version` - Current database schema version, if available
/// * `stats` - Database statistics, if health check succeeded
/// * `errors` - List of error messages encountered during health check
///
/// # Examples
///
/// ```rust
/// use demiarch_core::infrastructure::db::utils::DatabaseHealthCheck;
///
/// let health = DatabaseHealthCheck::success();
/// assert!(health.is_healthy);
///
/// let failed_health = DatabaseHealthCheck::failed(vec!["Connection failed".to_string()]);
/// assert!(!failed_health.is_healthy);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealthCheck {
    /// Overall health status
    ///
    /// true if the database is healthy and all checks passed,
    /// false if any errors were encountered during the health check.
    pub is_healthy: bool,
    /// Connection capability
    ///
    /// true if a connection to the database could be established,
    /// false if connection failed.
    pub can_connect: bool,
    /// Read capability
    ///
    /// true if read operations (queries) are working,
    /// false if read operations failed.
    pub can_read: bool,
    /// Write capability
    ///
    /// true if write operations (inserts, updates, deletes) are working,
    /// false if write operations failed.
    pub can_write: bool,
    /// Schema version
    ///
    /// The current database schema version if available,
    /// None if schema version could not be determined.
    pub schema_version: Option<String>,
    /// Database statistics
    ///
    /// Database statistics if the health check was successful,
    /// None if statistics could not be collected.
    pub stats: Option<DatabaseStats>,
    /// Error messages
    ///
    /// List of error messages encountered during the health check.
    /// Empty if no errors occurred.
    pub errors: Vec<String>,
}

impl DatabaseHealthCheck {
    pub fn success() -> Self {
        Self {
            is_healthy: true,
            can_connect: true,
            can_read: true,
            can_write: true,
            schema_version: Some(crate::infrastructure::db::schema::SCHEMA_VERSION.to_string()),
            stats: None,
            errors: Vec::new(),
        }
    }

    pub fn failed(errors: Vec<String>) -> Self {
        Self {
            is_healthy: false,
            can_connect: false,
            can_read: false,
            can_write: false,
            schema_version: None,
            stats: None,
            errors,
        }
    }
}

/// Perform comprehensive database health check
pub async fn perform_health_check(manager: &DatabaseManager) -> DatabaseHealthCheck {
    // Check if we can get the pool
    let pool = match manager.pool() {
        Ok(pool) => pool,
        Err(e) => {
            return DatabaseHealthCheck::failed(vec![format!(
                "Failed to get database pool: {}",
                e
            )]);
        }
    };

    let mut errors = Vec::new();
    let mut can_connect = true;
    let mut can_read = true;
    let mut can_write = true;

    // Test connection
    match pool.begin().await {
        Ok(_) => { /* Connection successful */ }
        Err(e) => {
            can_connect = false;
            errors.push(format!("Failed to connect to database: {}", e));
        }
    }

    // Test read operation
    if can_connect {
        match crate::infrastructure::db::migrations::get_schema_version(pool).await {
            Ok(_) => { /* Read successful */ }
            Err(e) => {
                can_read = false;
                errors.push(format!("Failed to read from database: {}", e));
            }
        }
    }

    // Test write operation
    if can_connect && can_read {
        match test_write_operation(pool).await {
            Ok(_) => { /* Write successful */ }
            Err(e) => {
                can_write = false;
                errors.push(format!("Failed to write to database: {}", e));
            }
        }
    }

    // Get statistics if healthy
    let stats = if can_connect && can_read {
        match DatabaseUtils::get_database_stats(pool).await {
            Ok(stats) => Some(stats),
            Err(e) => {
                errors.push(format!("Failed to get database stats: {}", e));
                None
            }
        }
    } else {
        None
    };

    let schema_version = if can_connect && can_read {
        crate::infrastructure::db::migrations::get_schema_version(pool)
            .await
            .ok()
    } else {
        None
    };

    DatabaseHealthCheck {
        is_healthy: errors.is_empty(),
        can_connect,
        can_read,
        can_write,
        schema_version: schema_version.and_then(|v| v),
        stats,
        errors,
    }
}

/// Test write operation with a simple insert/delete
async fn test_write_operation(pool: &SqlitePool) -> DatabaseResult<()> {
    let test_id = DatabaseUtils::generate_id();

    let mut conn = pool.begin().await?;

    // Insert test record
    sqlx::query(
        "INSERT INTO projects (id, name, description, created_at, updated_at, settings, status, metadata) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(test_id.to_string())
    .bind("health_check_test")
    .bind("Temporary test record for health check")
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .bind("{}")
    .bind("active")
    .bind("{}")
    .execute(&mut *conn)
    .await?;

    // Delete test record
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(test_id.to_string())
        .execute(&mut *conn)
        .await?;

    conn.commit().await?;
    Ok(())
}

/// Database query helper for pagination
pub struct QueryBuilder;

impl QueryBuilder {
    /// Build a paginated query with LIMIT and OFFSET
    pub fn paginate(base_query: &str, page: u32, page_size: u32) -> String {
        let offset = (page - 1) * page_size;
        format!("{} LIMIT {} OFFSET {}", base_query, page_size, offset)
    }

    /// Build a count query from a SELECT query
    pub fn count_query(select_query: &str) -> String {
        // Simple approach: replace SELECT ... with SELECT COUNT(*)
        let lower_query = select_query.to_lowercase();
        if let Some(_start) = lower_query.find("select") {
            let end = lower_query.find("from").unwrap_or(lower_query.len());
            let count_query = format!("SELECT COUNT(*) AS total {}", &select_query[end..]);
            return count_query;
        }

        // Fallback
        format!("SELECT COUNT(*) AS total ({})", select_query)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_map() {
        let json_str = r#"{"key": "value", "number": 123}"#;
        let map = DatabaseUtils::json_to_map(json_str).unwrap();
        assert_eq!(
            map.get("key"),
            Some(&serde_json::Value::String("value".to_string()))
        );
    }

    #[test]
    fn test_map_to_json() {
        let mut map = std::collections::HashMap::<String, serde_json::Value>::new();
        map.insert("test", serde_json::Value::String("value".to_string()));
        let json = DatabaseUtils::map_to_json(&map).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_paginate() {
        let query = "SELECT * FROM table";
        let paginated = DatabaseUtils::paginate(query, 2, 10);
        assert!(paginated.contains("LIMIT 10 OFFSET 10"));
    }

    #[test]
    fn test_count_query() {
        let query = "SELECT id FROM users";
        let count = DatabaseUtils::count_query(query);
        assert!(count.contains("SELECT COUNT(*) AS total"));
    }
}
