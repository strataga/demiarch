//! Database migrations
//!
//! This module manages SQLite schema migrations for demiarch.
//! Migrations are versioned and applied automatically on database connection.

use sqlx::SqlitePool;

/// Current schema version
pub const CURRENT_VERSION: i32 = 2;

/// SQL for creating the migrations tracking table
const CREATE_MIGRATIONS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS _migrations (
        version INTEGER PRIMARY KEY NOT NULL,
        applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
"#;

/// Migration 1: Initial schema
const MIGRATION_V1: &str = r#"
    -- Projects table
    CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        framework TEXT NOT NULL DEFAULT '',
        repo_url TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived', 'deleted')),
        description TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
    CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name);

    -- Features table
    CREATE TABLE IF NOT EXISTS features (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        title TEXT NOT NULL,
        description TEXT,
        phase_id TEXT,
        status TEXT NOT NULL DEFAULT 'backlog' CHECK (status IN ('backlog', 'todo', 'in_progress', 'review', 'done')),
        priority INTEGER NOT NULL DEFAULT 0,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_features_project_id ON features(project_id);
    CREATE INDEX IF NOT EXISTS idx_features_status ON features(status);
    CREATE INDEX IF NOT EXISTS idx_features_phase_id ON features(phase_id);

    -- Phases table (for organizing features)
    CREATE TABLE IF NOT EXISTS phases (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        description TEXT,
        order_index INTEGER NOT NULL DEFAULT 0,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_phases_project_id ON phases(project_id);

    -- LLM cost tracking table
    CREATE TABLE IF NOT EXISTS llm_costs (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
        model TEXT NOT NULL,
        input_tokens INTEGER NOT NULL DEFAULT 0,
        output_tokens INTEGER NOT NULL DEFAULT 0,
        input_cost_usd REAL NOT NULL DEFAULT 0.0,
        output_cost_usd REAL NOT NULL DEFAULT 0.0,
        context TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_llm_costs_project_id ON llm_costs(project_id);
    CREATE INDEX IF NOT EXISTS idx_llm_costs_created_at ON llm_costs(created_at);
    CREATE INDEX IF NOT EXISTS idx_llm_costs_model ON llm_costs(model);

    -- Daily cost summaries (materialized for performance)
    CREATE TABLE IF NOT EXISTS daily_cost_summaries (
        date TEXT NOT NULL,
        project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
        model TEXT NOT NULL,
        total_cost_usd REAL NOT NULL DEFAULT 0.0,
        total_input_tokens INTEGER NOT NULL DEFAULT 0,
        total_output_tokens INTEGER NOT NULL DEFAULT 0,
        call_count INTEGER NOT NULL DEFAULT 0,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        PRIMARY KEY (date, project_id, model)
    );

    CREATE INDEX IF NOT EXISTS idx_daily_cost_summaries_date ON daily_cost_summaries(date);

    -- Encrypted API keys table
    CREATE TABLE IF NOT EXISTS encrypted_keys (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL UNIQUE,
        ciphertext TEXT NOT NULL,
        nonce TEXT NOT NULL,
        description TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_used_at TIMESTAMP
    );

    CREATE UNIQUE INDEX IF NOT EXISTS idx_encrypted_keys_name ON encrypted_keys(name);

    -- Conversation history table
    CREATE TABLE IF NOT EXISTS conversations (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        title TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_conversations_project_id ON conversations(project_id);

    -- Messages in conversations
    CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY NOT NULL,
        conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
        role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
        content TEXT NOT NULL,
        model TEXT,
        tokens_used INTEGER,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
    CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);

    -- Checkpoints for code safety/recovery
    CREATE TABLE IF NOT EXISTS checkpoints (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        description TEXT,
        file_path TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        content TEXT NOT NULL,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_checkpoints_project_id ON checkpoints(project_id);
    CREATE INDEX IF NOT EXISTS idx_checkpoints_file_path ON checkpoints(file_path);
"#;

/// Migration 2: Documents table for PRD and architecture document generation
const MIGRATION_V2: &str = r#"
    -- Documents table for auto-generated PRDs, architecture docs, etc.
    CREATE TABLE IF NOT EXISTS documents (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        doc_type TEXT NOT NULL CHECK (doc_type IN ('prd', 'architecture', 'design', 'tech_spec', 'custom')),
        title TEXT NOT NULL,
        description TEXT,
        content TEXT NOT NULL,
        format TEXT NOT NULL DEFAULT 'markdown' CHECK (format IN ('markdown', 'json')),
        version INTEGER NOT NULL DEFAULT 1,
        status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'review', 'final', 'archived')),
        model_used TEXT,
        tokens_used INTEGER,
        generation_cost_usd REAL,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_documents_project_id ON documents(project_id);
    CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
    CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);

    -- Document versions for tracking changes
    CREATE TABLE IF NOT EXISTS document_versions (
        id TEXT PRIMARY KEY NOT NULL,
        document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
        version_number INTEGER NOT NULL,
        content TEXT NOT NULL,
        change_summary TEXT,
        model_used TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_document_versions_document_id ON document_versions(document_id);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_document_versions_unique ON document_versions(document_id, version_number);
"#;

/// Get the current schema version from the database
async fn get_current_version(pool: &SqlitePool) -> anyhow::Result<i32> {
    // Ensure migrations table exists
    sqlx::raw_sql(CREATE_MIGRATIONS_TABLE).execute(pool).await?;

    // Get the latest version
    let row: Option<(i32,)> = sqlx::query_as("SELECT MAX(version) FROM _migrations")
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|(v,)| v).unwrap_or(0))
}

/// Record that a migration has been applied
async fn record_migration(pool: &SqlitePool, version: i32) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO _migrations (version) VALUES (?)")
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

/// Run all pending migrations
pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    let current_version = get_current_version(pool).await?;

    tracing::info!(
        current_version = current_version,
        target_version = CURRENT_VERSION,
        "Checking database migrations"
    );

    if current_version >= CURRENT_VERSION {
        tracing::debug!("Database is up to date");
        return Ok(());
    }

    // Apply migrations in order
    if current_version < 1 {
        tracing::info!("Applying migration v1: Initial schema");
        sqlx::raw_sql(MIGRATION_V1).execute(pool).await?;
        record_migration(pool, 1).await?;
    }

    if current_version < 2 {
        tracing::info!("Applying migration v2: Documents table");
        sqlx::raw_sql(MIGRATION_V2).execute(pool).await?;
        record_migration(pool, 2).await?;
    }

    tracing::info!("Database migrations completed");
    Ok(())
}

/// Check if the database needs migrations
pub async fn needs_migration(pool: &SqlitePool) -> anyhow::Result<bool> {
    let current_version = get_current_version(pool).await?;
    Ok(current_version < CURRENT_VERSION)
}

/// Get migration status information
pub async fn migration_status(pool: &SqlitePool) -> anyhow::Result<MigrationStatus> {
    let current_version = get_current_version(pool).await?;
    Ok(MigrationStatus {
        current_version,
        target_version: CURRENT_VERSION,
        needs_migration: current_version < CURRENT_VERSION,
    })
}

/// Migration status information
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    /// Current schema version in the database
    pub current_version: i32,
    /// Target schema version (latest)
    pub target_version: i32,
    /// Whether migrations need to be run
    pub needs_migration: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool")
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let pool = create_test_pool().await;

        // Should start with no migrations
        let status = migration_status(&pool).await.unwrap();
        assert_eq!(status.current_version, 0);
        assert!(status.needs_migration);

        // Run migrations
        run_migrations(&pool).await.unwrap();

        // Should be at current version
        let status = migration_status(&pool).await.unwrap();
        assert_eq!(status.current_version, CURRENT_VERSION);
        assert!(!status.needs_migration);
    }

    #[tokio::test]
    async fn test_migrations_idempotent() {
        let pool = create_test_pool().await;

        // Run migrations twice
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Should still be at current version
        let status = migration_status(&pool).await.unwrap();
        assert_eq!(status.current_version, CURRENT_VERSION);
    }

    #[tokio::test]
    async fn test_tables_created() {
        let pool = create_test_pool().await;
        run_migrations(&pool).await.unwrap();

        // Check that tables exist by querying them
        let tables = vec![
            "projects",
            "features",
            "phases",
            "llm_costs",
            "daily_cost_summaries",
            "encrypted_keys",
            "conversations",
            "messages",
            "checkpoints",
            "documents",
            "document_versions",
        ];

        for table in tables {
            let result: (i32,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table))
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("Table {} should exist", table));
            // Just checking the query succeeds, count should be 0
            assert_eq!(result.0, 0);
        }
    }
}
