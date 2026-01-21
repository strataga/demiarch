//! Database migrations
//!
//! This module manages SQLite schema migrations for demiarch.
//! Migrations are versioned and applied automatically on database connection.

use sqlx::SqlitePool;

/// Current schema version
pub const CURRENT_VERSION: i32 = 9;

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

/// Migration 5: Generated file tracking for user edit detection
///
/// Tracks files generated by the system along with their content hashes,
/// enabling detection of manual user modifications to generated code.
const MIGRATION_V5: &str = r#"
    -- Track generated files for edit detection
    CREATE TABLE IF NOT EXISTS generated_files (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        feature_id TEXT REFERENCES features(id) ON DELETE SET NULL,
        file_path TEXT NOT NULL,
        content_hash TEXT NOT NULL,           -- SHA-256 hash at generation time
        generation_timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_verified_hash TEXT,              -- Hash when last verified (may differ if edited)
        last_verified_at TIMESTAMP,
        edit_detected INTEGER NOT NULL DEFAULT 0,  -- 1 if user edit detected
        UNIQUE(project_id, file_path)
    );

    CREATE INDEX IF NOT EXISTS idx_generated_files_project_id ON generated_files(project_id);
    CREATE INDEX IF NOT EXISTS idx_generated_files_feature_id ON generated_files(feature_id);
    CREATE INDEX IF NOT EXISTS idx_generated_files_file_path ON generated_files(file_path);
    CREATE INDEX IF NOT EXISTS idx_generated_files_edit_detected ON generated_files(edit_detected);
"#;

/// Migration 6: Learned skills system for autonomous knowledge extraction
///
/// Stores skills extracted from successful agent interactions. Skills are
/// patterns, techniques, and reusable knowledge that can be applied to
/// future tasks. Includes usage tracking for confidence adjustment.
const MIGRATION_V6: &str = r#"
    -- Learned skills table for storing extracted knowledge
    CREATE TABLE IF NOT EXISTS learned_skills (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        description TEXT NOT NULL,
        category TEXT NOT NULL CHECK (category IN (
            'code_generation', 'refactoring', 'testing', 'debugging',
            'architecture', 'performance', 'security', 'documentation',
            'api_design', 'database', 'error_handling', 'other'
        )),

        -- Pattern data (JSON)
        pattern_type TEXT NOT NULL,
        pattern_template TEXT NOT NULL,
        pattern_variables TEXT,              -- JSON array of variables
        pattern_applicability TEXT,          -- JSON array of conditions
        pattern_limitations TEXT,            -- JSON array of limitations

        -- Source context
        source_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
        source_feature_id TEXT REFERENCES features(id) ON DELETE SET NULL,
        source_agent_type TEXT,
        source_original_task TEXT,
        source_model_used TEXT,
        source_tokens_used INTEGER,

        -- Classification
        confidence TEXT NOT NULL DEFAULT 'medium' CHECK (confidence IN ('low', 'medium', 'high')),
        tags TEXT,                           -- JSON array of tags

        -- Usage statistics
        times_used INTEGER NOT NULL DEFAULT 0,
        success_count INTEGER NOT NULL DEFAULT 0,
        failure_count INTEGER NOT NULL DEFAULT 0,
        last_used_at TIMESTAMP,

        -- Metadata (JSON)
        metadata TEXT,

        -- Timestamps
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    -- Indexes for efficient querying
    CREATE INDEX IF NOT EXISTS idx_learned_skills_category ON learned_skills(category);
    CREATE INDEX IF NOT EXISTS idx_learned_skills_confidence ON learned_skills(confidence);
    CREATE INDEX IF NOT EXISTS idx_learned_skills_source_project_id ON learned_skills(source_project_id);
    CREATE INDEX IF NOT EXISTS idx_learned_skills_times_used ON learned_skills(times_used);
    CREATE INDEX IF NOT EXISTS idx_learned_skills_created_at ON learned_skills(created_at);

    -- Full-text search on name and description for skill discovery
    CREATE VIRTUAL TABLE IF NOT EXISTS learned_skills_fts USING fts5(
        name, description, tags,
        content='learned_skills',
        content_rowid='rowid'
    );

    -- Triggers to keep FTS index in sync
    CREATE TRIGGER IF NOT EXISTS learned_skills_ai AFTER INSERT ON learned_skills BEGIN
        INSERT INTO learned_skills_fts(rowid, name, description, tags)
        VALUES (NEW.rowid, NEW.name, NEW.description, NEW.tags);
    END;

    CREATE TRIGGER IF NOT EXISTS learned_skills_ad AFTER DELETE ON learned_skills BEGIN
        INSERT INTO learned_skills_fts(learned_skills_fts, rowid, name, description, tags)
        VALUES ('delete', OLD.rowid, OLD.name, OLD.description, OLD.tags);
    END;

    CREATE TRIGGER IF NOT EXISTS learned_skills_au AFTER UPDATE ON learned_skills BEGIN
        INSERT INTO learned_skills_fts(learned_skills_fts, rowid, name, description, tags)
        VALUES ('delete', OLD.rowid, OLD.name, OLD.description, OLD.tags);
        INSERT INTO learned_skills_fts(rowid, name, description, tags)
        VALUES (NEW.rowid, NEW.name, NEW.description, NEW.tags);
    END;
"#;

/// Migration 7: Semantic search embeddings for skills
///
/// Adds vector embeddings to learned skills for semantic similarity search.
/// Embeddings are stored as BLOB (binary float arrays) for compact storage.
/// A separate embeddings table allows for multiple embedding types/models.
const MIGRATION_V7: &str = r#"
    -- Skill embeddings table for semantic search
    CREATE TABLE IF NOT EXISTS skill_embeddings (
        id TEXT PRIMARY KEY NOT NULL,
        skill_id TEXT NOT NULL REFERENCES learned_skills(id) ON DELETE CASCADE,
        embedding_model TEXT NOT NULL,           -- Model used (e.g., "openai/text-embedding-3-small")
        embedding_version INTEGER NOT NULL DEFAULT 1,  -- Version for model updates
        embedding BLOB NOT NULL,                 -- Binary float array (f32 little-endian)
        dimensions INTEGER NOT NULL,             -- Vector dimensions (e.g., 1536)
        text_hash TEXT NOT NULL,                 -- Hash of embedded text for cache invalidation
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE(skill_id, embedding_model)
    );

    -- Indexes for efficient querying
    CREATE INDEX IF NOT EXISTS idx_skill_embeddings_skill_id ON skill_embeddings(skill_id);
    CREATE INDEX IF NOT EXISTS idx_skill_embeddings_model ON skill_embeddings(embedding_model);
    CREATE INDEX IF NOT EXISTS idx_skill_embeddings_text_hash ON skill_embeddings(text_hash);

    -- Add embedding_text column to learned_skills for caching the searchable text
    -- This combines name + description + tags for embedding generation
    ALTER TABLE learned_skills ADD COLUMN embedding_text TEXT;

    -- Note: embedding_text triggers removed - embedding_text is now computed on read
    -- This avoids potential trigger conflicts with FTS5 external content tables
"#;

/// Migration 8: Global session management
///
/// Provides global session tracking across multiple projects for workspace
/// continuity, session recovery, and cross-project context switching.
const MIGRATION_V8: &str = r#"
    -- Sessions table for global session management
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY NOT NULL,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_activity TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

        -- Context
        current_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
        current_feature_id TEXT REFERENCES features(id) ON DELETE SET NULL,

        -- State
        status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'completed', 'abandoned')),
        phase TEXT NOT NULL DEFAULT 'unknown' CHECK (phase IN ('discovery', 'planning', 'building', 'testing', 'review', 'unknown')),
        description TEXT,

        -- Recovery
        last_checkpoint_id TEXT REFERENCES checkpoints(id) ON DELETE SET NULL,

        -- Metadata (JSON for extensibility)
        metadata TEXT
    );

    -- Indexes for efficient querying
    CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
    CREATE INDEX IF NOT EXISTS idx_sessions_current_project_id ON sessions(current_project_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
    CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions(last_activity);

    -- Session events for audit trail
    CREATE TABLE IF NOT EXISTS session_events (
        id TEXT PRIMARY KEY NOT NULL,
        session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        event_type TEXT NOT NULL CHECK (event_type IN (
            'started', 'paused', 'resumed', 'completed', 'abandoned',
            'project_switched', 'feature_switched', 'phase_changed',
            'checkpoint_created', 'error', 'custom'
        )),
        data TEXT,  -- JSON event data
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    -- Indexes for event querying
    CREATE INDEX IF NOT EXISTS idx_session_events_session_id ON session_events(session_id);
    CREATE INDEX IF NOT EXISTS idx_session_events_event_type ON session_events(event_type);
    CREATE INDEX IF NOT EXISTS idx_session_events_created_at ON session_events(created_at);
"#;

/// Migration 9: Cross-project search with privacy controls
///
/// Enables searching across multiple projects with opt-in privacy settings.
/// Each project can control whether it's searchable from other projects.
const MIGRATION_V9: &str = r#"
    -- Project search settings for cross-project search privacy
    CREATE TABLE IF NOT EXISTS project_search_settings (
        project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

        -- Privacy controls
        allow_cross_project_search INTEGER NOT NULL DEFAULT 1,  -- Opt-in by default
        searchable_by_all INTEGER NOT NULL DEFAULT 1,           -- Can be searched by any project

        -- Granular controls (JSON array of project IDs)
        allowed_searchers TEXT,     -- If not null, only these projects can search this one
        excluded_searchers TEXT,    -- These projects are explicitly blocked

        -- Search scope controls
        include_features INTEGER NOT NULL DEFAULT 1,
        include_conversations INTEGER NOT NULL DEFAULT 0,  -- Off by default for privacy
        include_documents INTEGER NOT NULL DEFAULT 1,
        include_checkpoints INTEGER NOT NULL DEFAULT 0,    -- Off by default
        include_skills INTEGER NOT NULL DEFAULT 1,

        -- Metadata
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    -- Index for quick lookups
    CREATE INDEX IF NOT EXISTS idx_project_search_settings_cross_project
        ON project_search_settings(allow_cross_project_search);

    -- Cross-project search history for audit trail
    CREATE TABLE IF NOT EXISTS cross_project_search_log (
        id TEXT PRIMARY KEY NOT NULL,
        searcher_project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        query TEXT NOT NULL,
        searched_project_ids TEXT NOT NULL,  -- JSON array of project IDs searched
        result_count INTEGER NOT NULL DEFAULT 0,
        search_scope TEXT NOT NULL,          -- JSON array of entity types searched
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_cross_project_search_log_searcher
        ON cross_project_search_log(searcher_project_id);
    CREATE INDEX IF NOT EXISTS idx_cross_project_search_log_created_at
        ON cross_project_search_log(created_at);

    -- Full-text search index for features (extends existing FTS pattern)
    CREATE VIRTUAL TABLE IF NOT EXISTS features_fts USING fts5(
        title, description, acceptance_criteria,
        content='features',
        content_rowid='rowid'
    );

    -- Triggers to keep features FTS index in sync
    CREATE TRIGGER IF NOT EXISTS features_ai AFTER INSERT ON features BEGIN
        INSERT INTO features_fts(rowid, title, description, acceptance_criteria)
        VALUES (NEW.rowid, NEW.title, NEW.description, NEW.acceptance_criteria);
    END;

    CREATE TRIGGER IF NOT EXISTS features_ad AFTER DELETE ON features BEGIN
        INSERT INTO features_fts(features_fts, rowid, title, description, acceptance_criteria)
        VALUES ('delete', OLD.rowid, OLD.title, OLD.description, OLD.acceptance_criteria);
    END;

    CREATE TRIGGER IF NOT EXISTS features_au AFTER UPDATE ON features BEGIN
        INSERT INTO features_fts(features_fts, rowid, title, description, acceptance_criteria)
        VALUES ('delete', OLD.rowid, OLD.title, OLD.description, OLD.acceptance_criteria);
        INSERT INTO features_fts(rowid, title, description, acceptance_criteria)
        VALUES (NEW.rowid, NEW.title, NEW.description, NEW.acceptance_criteria);
    END;

    -- Full-text search index for documents
    CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
        title, description, content,
        content='documents',
        content_rowid='rowid'
    );

    -- Triggers to keep documents FTS index in sync
    CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
        INSERT INTO documents_fts(rowid, title, description, content)
        VALUES (NEW.rowid, NEW.title, NEW.description, NEW.content);
    END;

    CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
        INSERT INTO documents_fts(documents_fts, rowid, title, description, content)
        VALUES ('delete', OLD.rowid, OLD.title, OLD.description, OLD.content);
    END;

    CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
        INSERT INTO documents_fts(documents_fts, rowid, title, description, content)
        VALUES ('delete', OLD.rowid, OLD.title, OLD.description, OLD.content);
        INSERT INTO documents_fts(rowid, title, description, content)
        VALUES (NEW.rowid, NEW.title, NEW.description, NEW.content);
    END;

    -- Full-text search index for messages (conversations)
    CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
        content,
        content='messages',
        content_rowid='rowid'
    );

    -- Triggers to keep messages FTS index in sync
    CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
        INSERT INTO messages_fts(rowid, content)
        VALUES (NEW.rowid, NEW.content);
    END;

    CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
        INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES ('delete', OLD.rowid, OLD.content);
    END;

    CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
        INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES ('delete', OLD.rowid, OLD.content);
        INSERT INTO messages_fts(rowid, content)
        VALUES (NEW.rowid, NEW.content);
    END;
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

    if current_version < 5 {
        tracing::info!("Applying migration v5: Generated file tracking for edit detection");
        sqlx::raw_sql(MIGRATION_V5).execute(pool).await?;
        record_migration(pool, 5).await?;
    }

    if current_version < 6 {
        tracing::info!("Applying migration v6: Learned skills system");
        sqlx::raw_sql(MIGRATION_V6).execute(pool).await?;
        record_migration(pool, 6).await?;
    }

    if current_version < 7 {
        tracing::info!("Applying migration v7: Semantic search embeddings");
        sqlx::raw_sql(MIGRATION_V7).execute(pool).await?;
        record_migration(pool, 7).await?;
    }

    if current_version < 8 {
        tracing::info!("Applying migration v8: Global session management");
        sqlx::raw_sql(MIGRATION_V8).execute(pool).await?;
        record_migration(pool, 8).await?;
    }

    if current_version < 9 {
        tracing::info!("Applying migration v9: Cross-project search with privacy controls");
        sqlx::raw_sql(MIGRATION_V9).execute(pool).await?;
        record_migration(pool, 9).await?;
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
            "generated_files",
            "learned_skills",
            "skill_embeddings",
            "sessions",
            "session_events",
            "project_search_settings",
            "cross_project_search_log",
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
