//! Integration tests for SQLite database functionality
//!
//! These tests verify the full database workflow including:
//! - Database creation with proper permissions
//! - Connection pooling
//! - Migration execution
//! - CRUD operations
//! - Health checks

use demiarch_core::infrastructure::db::{
    get_connection_stats, DatabaseConfig, DatabaseManager, MigrationManager,
};
use sqlx::Row;
use tempfile::TempDir;

/// Helper to create a test database manager
async fn create_test_db() -> (TempDir, DatabaseManager) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.sqlite");

    let config = DatabaseConfig {
        path: db_path,
        max_connections: 5,
        min_connections: 1,
        connect_timeout: 10,
    };

    let mut manager = DatabaseManager::new(config);
    manager.initialize().await.unwrap();

    (temp_dir, manager)
}

#[tokio::test]
async fn test_database_manager_initialization() {
    let (_temp_dir, manager) = create_test_db().await;

    // Verify pool is available
    let pool = manager.pool().expect("Pool should be initialized");

    // Verify we can execute a query
    let result = sqlx::query("SELECT 1 as value").fetch_one(pool).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_migrations_run_successfully() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Check schema version
    let migration_manager = MigrationManager::new();
    let version = migration_manager.get_current_version(pool).await.unwrap();

    assert!(version.is_some());
    assert_eq!(version.unwrap(), "001");
}

#[tokio::test]
async fn test_all_tables_created() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Query for all tables
    let tables: Vec<String> = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    // Verify all expected tables exist
    let expected_tables = vec![
        "projects",
        "conversations",
        "agents",
        "skills",
        "code_generation",
        "llm_calls",
        "checkpoints",
        "plugins",
        "sessions",
        "schema_version",
    ];

    for table in expected_tables {
        assert!(
            tables.contains(&table.to_string()),
            "Table '{}' should exist",
            table
        );
    }
}

#[tokio::test]
async fn test_connection_pool_settings() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    let stats = get_connection_stats(pool);

    // Verify pool has connections
    assert!(stats.total_connections >= 1);
}

#[tokio::test]
async fn test_project_crud_operations() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    let project_id = uuid::Uuid::new_v4().to_string();

    // Create a project
    sqlx::query(
        "INSERT INTO projects (id, name, description, settings, status, metadata) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_id)
    .bind("Test Project")
    .bind("A test project for integration testing")
    .bind("{}")
    .bind("active")
    .bind("{}")
    .execute(pool)
    .await
    .unwrap();

    // Read the project
    let row = sqlx::query("SELECT * FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("name"), "Test Project");
    assert_eq!(row.get::<String, _>("status"), "active");

    // Update the project
    sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
        .bind("Updated Project")
        .bind(&project_id)
        .execute(pool)
        .await
        .unwrap();

    let row = sqlx::query("SELECT name FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("name"), "Updated Project");

    // Delete the project
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&project_id)
        .execute(pool)
        .await
        .unwrap();

    let count = sqlx::query("SELECT COUNT(*) as count FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(count.get::<i64, _>("count"), 0);
}

#[tokio::test]
async fn test_foreign_key_constraints() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Create a project first
    let project_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO projects (id, name, description, settings, status, metadata) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_id)
    .bind("FK Test Project")
    .bind("Testing foreign keys")
    .bind("{}")
    .bind("active")
    .bind("{}")
    .execute(pool)
    .await
    .unwrap();

    // Create an agent linked to the project
    let agent_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO agents (id, project_id, name, type, configuration, state, status) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&agent_id)
    .bind(&project_id)
    .bind("Test Agent")
    .bind("code_gen")
    .bind("{}")
    .bind("{}")
    .bind("active")
    .execute(pool)
    .await
    .unwrap();

    // Delete the project - should cascade to agent
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&project_id)
        .execute(pool)
        .await
        .unwrap();

    // Agent should be deleted due to ON DELETE CASCADE
    let count = sqlx::query("SELECT COUNT(*) as count FROM agents WHERE id = ?")
        .bind(&agent_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(count.get::<i64, _>("count"), 0);
}

#[tokio::test]
async fn test_database_health_check() {
    let (_temp_dir, manager) = create_test_db().await;

    let health = manager.health_check().await.unwrap();

    assert!(health.is_healthy);
    assert!(health.table_count >= 9); // At least 9 tables plus schema_version
    assert!(health.connection_count >= 1);
}

#[tokio::test]
async fn test_wal_mode_enabled() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Check that WAL mode is enabled
    let row = sqlx::query("PRAGMA journal_mode")
        .fetch_one(pool)
        .await
        .unwrap();

    let mode: String = row.get(0);
    assert_eq!(mode.to_lowercase(), "wal");
}

#[tokio::test]
async fn test_foreign_keys_enabled() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Check that foreign keys are enabled
    let row = sqlx::query("PRAGMA foreign_keys")
        .fetch_one(pool)
        .await
        .unwrap();

    let fk_enabled: i32 = row.get(0);
    assert_eq!(fk_enabled, 1);
}

#[tokio::test]
async fn test_indexes_created() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Query for all indexes
    let indexes: Vec<String> =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%'")
            .fetch_all(pool)
            .await
            .unwrap()
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

    // Verify key indexes exist
    let expected_indexes = vec![
        "idx_conversations_project_id",
        "idx_agents_project_id",
        "idx_llm_calls_project_id",
        "idx_llm_calls_created_at",
        "idx_sessions_expires_at",
    ];

    for index in expected_indexes {
        assert!(
            indexes.contains(&index.to_string()),
            "Index '{}' should exist",
            index
        );
    }
}

#[tokio::test]
async fn test_concurrent_connections() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("concurrent.sqlite");

    let config = DatabaseConfig {
        path: db_path,
        max_connections: 10,
        min_connections: 2,
        connect_timeout: 30,
    };

    let mut manager = DatabaseManager::new(config);
    manager.initialize().await.unwrap();
    let pool = manager.pool().unwrap();

    // Spawn multiple concurrent tasks
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let project_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO projects (id, name, description, settings, status, metadata) VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(&project_id)
                .bind(format!("Concurrent Project {}", i))
                .bind("Testing concurrent access")
                .bind("{}")
                .bind("active")
                .bind("{}")
                .execute(&pool)
                .await
                .unwrap();
                project_id
            })
        })
        .collect();

    // Wait for all tasks to complete
    let mut project_ids = Vec::new();
    for handle in handles {
        project_ids.push(handle.await.unwrap());
    }

    // Verify all projects were created
    let count = sqlx::query("SELECT COUNT(*) as count FROM projects")
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(count.get::<i64, _>("count"), 5);
}

#[tokio::test]
async fn test_llm_call_tracking() {
    let (_temp_dir, manager) = create_test_db().await;
    let pool = manager.pool().unwrap();

    // Create a project first
    let project_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO projects (id, name, description, settings, status, metadata) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_id)
    .bind("LLM Test Project")
    .bind("Testing LLM call tracking")
    .bind("{}")
    .bind("active")
    .bind("{}")
    .execute(pool)
    .await
    .unwrap();

    // Record an LLM call
    let call_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO llm_calls (id, project_id, model, provider, prompt_tokens, completion_tokens, total_tokens, cost_usd, duration_ms, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&call_id)
    .bind(&project_id)
    .bind("claude-sonnet-4-20250514")
    .bind("anthropic")
    .bind(100)
    .bind(200)
    .bind(300)
    .bind(0.015)
    .bind(1500)
    .bind("success")
    .execute(pool)
    .await
    .unwrap();

    // Query total cost for the project
    let row = sqlx::query("SELECT SUM(cost_usd) as total_cost FROM llm_calls WHERE project_id = ?")
        .bind(&project_id)
        .fetch_one(pool)
        .await
        .unwrap();

    let total_cost: f64 = row.get("total_cost");
    assert!((total_cost - 0.015).abs() < 0.001);
}
