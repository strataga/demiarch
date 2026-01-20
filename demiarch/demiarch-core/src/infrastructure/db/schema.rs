//! Database schema definitions and SQL statements

/// Database schema version
pub const SCHEMA_VERSION: &str = "001";

/// Create all database tables
pub fn create_tables_sql() -> &'static str {
    r#"
    -- Projects table - stores project metadata and configuration
    CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        settings TEXT NOT NULL DEFAULT '{}',
        status TEXT NOT NULL DEFAULT 'active',
        metadata TEXT NOT NULL DEFAULT '{}'
    );

    -- Conversations table - stores chat/conversation history
    CREATE TABLE IF NOT EXISTS conversations (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        title TEXT NOT NULL,
        messages TEXT NOT NULL DEFAULT '[]',
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        metadata TEXT NOT NULL DEFAULT '{}',
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Agents table - stores agent configurations and states
    CREATE TABLE IF NOT EXISTS agents (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        type TEXT NOT NULL,
        configuration TEXT NOT NULL DEFAULT '{}',
        state TEXT NOT NULL DEFAULT '{}',
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        status TEXT NOT NULL DEFAULT 'active',
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Skills table - stores learned skills and capabilities
    CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        category TEXT NOT NULL,
        code TEXT NOT NULL,
        metadata TEXT NOT NULL DEFAULT '{}',
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Code generation table - stores generated code and artifacts
    CREATE TABLE IF NOT EXISTS code_generation (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        conversation_id TEXT,
        agent_id TEXT,
        type TEXT NOT NULL,
        language TEXT NOT NULL,
        code TEXT NOT NULL,
        file_path TEXT,
        dependencies TEXT NOT NULL DEFAULT '[]',
        metadata TEXT NOT NULL DEFAULT '{}',
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
        FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
    );

    -- LLM calls table - stores LLM API call logs for cost tracking
    CREATE TABLE IF NOT EXISTS llm_calls (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        conversation_id TEXT,
        agent_id TEXT,
        model TEXT NOT NULL,
        provider TEXT NOT NULL,
        prompt_tokens INTEGER NOT NULL DEFAULT 0,
        completion_tokens INTEGER NOT NULL DEFAULT 0,
        total_tokens INTEGER NOT NULL DEFAULT 0,
        cost_usd REAL NOT NULL DEFAULT 0.0,
        duration_ms INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'success',
        error_message TEXT,
        request_text TEXT,
        response_text TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
        FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
    );

    -- Checkpoints table - stores recovery checkpoints
    CREATE TABLE IF NOT EXISTS checkpoints (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        type TEXT NOT NULL,
        data TEXT NOT NULL,
        metadata TEXT NOT NULL DEFAULT '{}',
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Plugins table - stores plugin configurations and state
    CREATE TABLE IF NOT EXISTS plugins (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        version TEXT NOT NULL,
        type TEXT NOT NULL,
        configuration TEXT NOT NULL DEFAULT '{}',
        state TEXT NOT NULL DEFAULT '{}',
        enabled BOOLEAN NOT NULL DEFAULT true,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Sessions table - stores user session management
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY NOT NULL,
        project_id TEXT NOT NULL,
        user_id TEXT,
        session_data TEXT NOT NULL DEFAULT '{}',
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        expires_at TIMESTAMP,
        last_accessed TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        status TEXT NOT NULL DEFAULT 'active',
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- Create indexes for better query performance
    CREATE INDEX IF NOT EXISTS idx_conversations_project_id ON conversations(project_id);
    CREATE INDEX IF NOT EXISTS idx_agents_project_id ON agents(project_id);
    CREATE INDEX IF NOT EXISTS idx_skills_project_id ON skills(project_id);
    CREATE INDEX IF NOT EXISTS idx_code_generation_project_id ON code_generation(project_id);
    CREATE INDEX IF NOT EXISTS idx_llm_calls_project_id ON llm_calls(project_id);
    CREATE INDEX IF NOT EXISTS idx_llm_calls_created_at ON llm_calls(created_at);
    CREATE INDEX IF NOT EXISTS idx_checkpoints_project_id ON checkpoints(project_id);
    CREATE INDEX IF NOT EXISTS idx_plugins_project_id ON plugins(project_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON sessions(project_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
    "#
}

/// Drop all database tables (for testing/reset purposes)
pub fn drop_tables_sql() -> &'static str {
    r#"
    DROP TABLE IF EXISTS sessions;
    DROP TABLE IF EXISTS plugins;
    DROP TABLE IF EXISTS checkpoints;
    DROP TABLE IF EXISTS llm_calls;
    DROP TABLE IF EXISTS code_generation;
    DROP TABLE IF EXISTS skills;
    DROP TABLE IF EXISTS agents;
    DROP TABLE IF EXISTS conversations;
    DROP TABLE IF EXISTS projects;
    "#
}

/// Create schema version table
pub fn create_schema_version_table_sql() -> &'static str {
    r#"
    CREATE TABLE IF NOT EXISTS schema_version (
        version TEXT PRIMARY KEY NOT NULL,
        applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        description TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_schema_version_version ON schema_version(version);
    "#
}

/// Insert schema version record
pub fn insert_schema_version_sql(version: &str, description: &str) -> String {
    format!(
        "INSERT OR REPLACE INTO schema_version (version, description) VALUES ('{}', '{}')",
        version, description
    )
}

/// Get current schema version
pub fn get_schema_version_sql() -> &'static str {
    "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert_eq!(SCHEMA_VERSION, "001");
    }

    #[test]
    fn test_create_tables_sql() {
        let sql = create_tables_sql();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS projects"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS conversations"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS agents"));
        assert!(sql.contains("CREATE INDEX"));
    }

    #[test]
    fn test_drop_tables_sql() {
        let sql = drop_tables_sql();
        assert!(sql.contains("DROP TABLE IF EXISTS"));
        assert!(sql.contains("projects"));
        assert!(sql.contains("conversations"));
        assert!(sql.contains("sessions"));
    }

    #[test]
    fn test_create_schema_version_table_sql() {
        let sql = create_schema_version_table_sql();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS schema_version"));
        assert!(sql.contains("version TEXT PRIMARY KEY"));
        assert!(sql.contains("applied_at TIMESTAMP"));
    }

    #[test]
    fn test_insert_schema_version_sql() {
        let sql = insert_schema_version_sql("001", "Initial schema");
        assert!(sql.contains("INSERT OR REPLACE INTO schema_version"));
        assert!(sql.contains("001"));
        assert!(sql.contains("Initial schema"));
    }

    #[test]
    fn test_get_schema_version_sql() {
        let sql = get_schema_version_sql();
        assert!(sql.contains("SELECT version FROM schema_version"));
        assert!(sql.contains("ORDER BY version DESC"));
    }
}
