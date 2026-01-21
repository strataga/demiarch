//! Database migrations
//!
//! This module manages SQLite schema migrations for demiarch.
//! Migrations are versioned and applied automatically on database connection.

use sqlx::SqlitePool;

/// Current schema version
pub const CURRENT_VERSION: i32 = 4;

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
#[allow(dead_code)]
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

/// Migration 3: Phase planning and feature breakdown enhancements
const MIGRATION_V3: &str = r#"
    -- Update phases table to support phase planning workflow
    ALTER TABLE phases ADD COLUMN status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'complete', 'skipped'));

    -- Add acceptance_criteria and labels columns to features table
    ALTER TABLE features ADD COLUMN acceptance_criteria TEXT;
    ALTER TABLE features ADD COLUMN labels TEXT;

    -- Create phase_templates table for default phase structures
    CREATE TABLE IF NOT EXISTS phase_templates (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        order_index INTEGER NOT NULL DEFAULT 0,
        is_default INTEGER NOT NULL DEFAULT 0,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    -- Insert default phase templates
    INSERT INTO phase_templates (id, name, description, order_index, is_default) VALUES
        ('discovery', 'Discovery', 'Requirements gathering and ideation', 0, 1),
        ('planning', 'Planning', 'Technical design and architecture', 1, 1),
        ('building', 'Building', 'Implementation and development', 2, 1),
        ('complete', 'Complete', 'Finished and deployed', 3, 1);

    -- Create feature_extraction_history for tracking LLM-generated feature breakdowns
    CREATE TABLE IF NOT EXISTS feature_extraction_history (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
        model_used TEXT NOT NULL,
        tokens_used INTEGER,
        cost_usd REAL,
        phases_created INTEGER NOT NULL DEFAULT 0,
        features_created INTEGER NOT NULL DEFAULT 0,
        raw_response TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_feature_extraction_history_project_id ON feature_extraction_history(project_id);
"#;

/// Migration 4: Enhanced checkpoints for automatic code safety
///
/// Transforms the basic file-level checkpoints table into a project-state
/// checkpoint system with Ed25519 signatures for integrity verification.
const MIGRATION_V4: &str = r#"
    -- Drop old checkpoints table and recreate with new schema
    -- The old schema stored individual file contents; new schema stores full project state
    DROP TABLE IF EXISTS checkpoints;

    -- Create enhanced checkpoints table for automatic code safety
    CREATE TABLE checkpoints (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        feature_id TEXT REFERENCES features(id) ON DELETE SET NULL,
        description TEXT NOT NULL,
        snapshot_data TEXT NOT NULL,  -- JSON blob containing project state
        size_bytes INTEGER NOT NULL,
        signature BLOB NOT NULL,      -- Ed25519 signature for integrity
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    -- Indexes for efficient querying
    CREATE INDEX IF NOT EXISTS idx_checkpoints_project_id ON checkpoints(project_id);
    CREATE INDEX IF NOT EXISTS idx_checkpoints_feature_id ON checkpoints(feature_id);
    CREATE INDEX IF NOT EXISTS idx_checkpoints_created_at ON checkpoints(created_at);
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

    if current_version < 3 {
        tracing::info!("Applying migration v3: Phase planning enhancements");
        sqlx::raw_sql(MIGRATION_V3).execute(pool).await?;
        record_migration(pool, 3).await?;
    }

    if current_version < 4 {
        tracing::info!("Applying migration v4: Enhanced checkpoints for code safety");
        sqlx::raw_sql(MIGRATION_V4).execute(pool).await?;
        record_migration(pool, 4).await?;
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
            "phase_templates",
            "feature_extraction_history",
        ];

        for table in tables {
            let result: (i32,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table))
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("Table {} should exist", table));
            // Just checking the query succeeds
            // phase_templates has 4 default rows, others should be 0
            if table == "phase_templates" {
                assert_eq!(result.0, 4, "phase_templates should have 4 default entries");
            } else {
                assert_eq!(result.0, 0, "Table {} should be empty", table);
            }
        }
    }
}
