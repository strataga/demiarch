//! Search repository for database operations
//!
//! Handles all database interactions for cross-project search.

use super::entity::{CrossProjectSearchLog, ProjectSearchSettings, SearchEntityType, SearchResult};
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Repository for search database operations
#[derive(Debug, Clone)]
pub struct SearchRepository {
    pool: SqlitePool,
}

impl SearchRepository {
    /// Create a new repository with the given connection pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ========== Project Search Settings ==========

    /// Get search settings for a project
    pub async fn get_settings(&self, project_id: Uuid) -> Result<Option<ProjectSearchSettings>> {
        let id = project_id.to_string();

        let row: Option<SettingsRow> = sqlx::query_as(
            r#"
            SELECT project_id, allow_cross_project_search, searchable_by_all,
                   allowed_searchers, excluded_searchers,
                   include_features, include_conversations, include_documents,
                   include_checkpoints, include_skills,
                   created_at, updated_at
            FROM project_search_settings
            WHERE project_id = ?
            "#,
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        match row {
            Some(row) => Ok(Some(row.into_settings()?)),
            None => Ok(None),
        }
    }

    /// Get or create default settings for a project
    pub async fn get_or_create_settings(&self, project_id: Uuid) -> Result<ProjectSearchSettings> {
        if let Some(settings) = self.get_settings(project_id).await? {
            return Ok(settings);
        }

        // Create default settings
        let settings = ProjectSearchSettings::default_for_project(project_id);
        self.save_settings(&settings).await?;
        Ok(settings)
    }

    /// Save search settings for a project
    pub async fn save_settings(&self, settings: &ProjectSearchSettings) -> Result<()> {
        let project_id = settings.project_id.to_string();
        let allowed_searchers = serde_json::to_string(&settings.allowed_searchers)
            .map_err(|e| Error::Parse(format!("Failed to serialize allowed_searchers: {}", e)))?;
        let excluded_searchers = serde_json::to_string(&settings.excluded_searchers)
            .map_err(|e| Error::Parse(format!("Failed to serialize excluded_searchers: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO project_search_settings (
                project_id, allow_cross_project_search, searchable_by_all,
                allowed_searchers, excluded_searchers,
                include_features, include_conversations, include_documents,
                include_checkpoints, include_skills,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(project_id) DO UPDATE SET
                allow_cross_project_search = excluded.allow_cross_project_search,
                searchable_by_all = excluded.searchable_by_all,
                allowed_searchers = excluded.allowed_searchers,
                excluded_searchers = excluded.excluded_searchers,
                include_features = excluded.include_features,
                include_conversations = excluded.include_conversations,
                include_documents = excluded.include_documents,
                include_checkpoints = excluded.include_checkpoints,
                include_skills = excluded.include_skills,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&project_id)
        .bind(settings.allow_cross_project_search)
        .bind(settings.searchable_by_all)
        .bind(&allowed_searchers)
        .bind(&excluded_searchers)
        .bind(settings.include_features)
        .bind(settings.include_conversations)
        .bind(settings.include_documents)
        .bind(settings.include_checkpoints)
        .bind(settings.include_skills)
        .bind(settings.created_at)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    /// Get all projects that can be searched by a given project
    pub async fn get_searchable_projects(&self, searcher_project_id: Uuid) -> Result<Vec<Uuid>> {
        let searcher_id = searcher_project_id.to_string();

        // Get all projects with their search settings
        // A project is searchable if:
        // 1. It has settings AND (searchable_by_all=true OR searcher is in allowed_searchers)
        // 2. AND searcher is NOT in excluded_searchers
        // OR
        // 3. It has no settings (use defaults: searchable_by_all=true)
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT p.id
            FROM projects p
            LEFT JOIN project_search_settings s ON p.id = s.project_id
            WHERE p.id != ?
              AND p.status = 'active'
              AND (
                -- No settings: default to searchable
                s.project_id IS NULL
                OR (
                    -- Has settings and is searchable
                    (s.searchable_by_all = 1 OR s.allowed_searchers LIKE '%' || ? || '%')
                    AND (s.excluded_searchers IS NULL OR s.excluded_searchers NOT LIKE '%' || ? || '%')
                )
              )
            "#,
        )
        .bind(&searcher_id)
        .bind(&searcher_id)
        .bind(&searcher_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter()
            .map(|(id,)| {
                Uuid::parse_str(&id).map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))
            })
            .collect()
    }

    // ========== Full-Text Search ==========

    /// Search features across specified projects
    pub async fn search_features(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }

        let project_ids_str: Vec<String> = project_ids.iter().map(|id| id.to_string()).collect();
        let placeholders: String = project_ids_str
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            r#"
            SELECT f.id, f.project_id, f.title, f.description,
                   f.created_at, p.name as project_name,
                   bm25(features_fts) as score
            FROM features_fts fts
            JOIN features f ON f.rowid = fts.rowid
            JOIN projects p ON f.project_id = p.id
            WHERE features_fts MATCH ?
              AND f.project_id IN ({})
            ORDER BY score
            LIMIT ? OFFSET ?
            "#,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, FeatureResultRow>(&sql);
        query_builder = query_builder.bind(query);
        for id in &project_ids_str {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(limit as i32).bind(offset as i32);

        let rows: Vec<FeatureResultRow> = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        rows.into_iter()
            .map(|row| row.into_search_result())
            .collect()
    }

    /// Search documents across specified projects
    pub async fn search_documents(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }

        let project_ids_str: Vec<String> = project_ids.iter().map(|id| id.to_string()).collect();
        let placeholders: String = project_ids_str
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            r#"
            SELECT d.id, d.project_id, d.title, d.description,
                   d.created_at, p.name as project_name,
                   bm25(documents_fts) as score
            FROM documents_fts fts
            JOIN documents d ON d.rowid = fts.rowid
            JOIN projects p ON d.project_id = p.id
            WHERE documents_fts MATCH ?
              AND d.project_id IN ({})
            ORDER BY score
            LIMIT ? OFFSET ?
            "#,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, DocumentResultRow>(&sql);
        query_builder = query_builder.bind(query);
        for id in &project_ids_str {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(limit as i32).bind(offset as i32);

        let rows: Vec<DocumentResultRow> = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        rows.into_iter()
            .map(|row| row.into_search_result())
            .collect()
    }

    /// Search messages (conversations) across specified projects
    pub async fn search_messages(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }

        let project_ids_str: Vec<String> = project_ids.iter().map(|id| id.to_string()).collect();
        let placeholders: String = project_ids_str
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            r#"
            SELECT m.id, c.project_id, c.title as conv_title, m.content,
                   m.created_at, p.name as project_name,
                   bm25(messages_fts) as score
            FROM messages_fts fts
            JOIN messages m ON m.rowid = fts.rowid
            JOIN conversations c ON m.conversation_id = c.id
            JOIN projects p ON c.project_id = p.id
            WHERE messages_fts MATCH ?
              AND c.project_id IN ({})
            ORDER BY score
            LIMIT ? OFFSET ?
            "#,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, MessageResultRow>(&sql);
        query_builder = query_builder.bind(query);
        for id in &project_ids_str {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(limit as i32).bind(offset as i32);

        let rows: Vec<MessageResultRow> = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        rows.into_iter()
            .map(|row| row.into_search_result())
            .collect()
    }

    /// Search learned skills (global or filtered by source project)
    pub async fn search_skills(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>> {
        // Skills can be global, so we search differently:
        // - If project_ids is empty, search all skills
        // - If project_ids is specified, filter by source_project_id
        let rows: Vec<SkillResultRow> = if project_ids.is_empty() {
            sqlx::query_as(
                r#"
                SELECT s.id, s.source_project_id, s.name, s.description,
                       s.created_at, p.name as project_name,
                       bm25(learned_skills_fts) as score
                FROM learned_skills_fts fts
                JOIN learned_skills s ON s.rowid = fts.rowid
                LEFT JOIN projects p ON s.source_project_id = p.id
                WHERE learned_skills_fts MATCH ?
                ORDER BY score
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(query)
            .bind(limit as i32)
            .bind(offset as i32)
            .fetch_all(&self.pool)
            .await
            .map_err(Error::DatabaseError)?
        } else {
            let project_ids_str: Vec<String> =
                project_ids.iter().map(|id| id.to_string()).collect();
            let placeholders: String = project_ids_str
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(",");

            let sql = format!(
                r#"
                SELECT s.id, s.source_project_id, s.name, s.description,
                       s.created_at, p.name as project_name,
                       bm25(learned_skills_fts) as score
                FROM learned_skills_fts fts
                JOIN learned_skills s ON s.rowid = fts.rowid
                LEFT JOIN projects p ON s.source_project_id = p.id
                WHERE learned_skills_fts MATCH ?
                  AND (s.source_project_id IS NULL OR s.source_project_id IN ({}))
                ORDER BY score
                LIMIT ? OFFSET ?
                "#,
                placeholders
            );

            let mut query_builder = sqlx::query_as::<_, SkillResultRow>(&sql);
            query_builder = query_builder.bind(query);
            for id in &project_ids_str {
                query_builder = query_builder.bind(id);
            }
            query_builder = query_builder.bind(limit as i32).bind(offset as i32);

            query_builder
                .fetch_all(&self.pool)
                .await
                .map_err(Error::DatabaseError)?
        };

        rows.into_iter()
            .map(|row| row.into_search_result())
            .collect()
    }

    // ========== Search Audit Log ==========

    /// Log a cross-project search for audit trail
    pub async fn log_search(&self, log: &CrossProjectSearchLog) -> Result<()> {
        let id = log.id.to_string();
        let searcher_project_id = log.searcher_project_id.to_string();
        let searched_project_ids =
            serde_json::to_string(&log.searched_project_ids).map_err(|e| {
                Error::Parse(format!("Failed to serialize searched_project_ids: {}", e))
            })?;
        let search_scope: Vec<String> = log
            .search_scope
            .iter()
            .map(|t| t.as_str().to_string())
            .collect();
        let search_scope_json = serde_json::to_string(&search_scope)
            .map_err(|e| Error::Parse(format!("Failed to serialize search_scope: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO cross_project_search_log (
                id, searcher_project_id, query, searched_project_ids,
                result_count, search_scope, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&searcher_project_id)
        .bind(&log.query)
        .bind(&searched_project_ids)
        .bind(log.result_count as i32)
        .bind(&search_scope_json)
        .bind(log.created_at)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    /// Get search history for a project
    pub async fn get_search_history(
        &self,
        project_id: Uuid,
        limit: u32,
    ) -> Result<Vec<CrossProjectSearchLog>> {
        let project_id_str = project_id.to_string();

        let rows: Vec<SearchLogRow> = sqlx::query_as(
            r#"
            SELECT id, searcher_project_id, query, searched_project_ids,
                   result_count, search_scope, created_at
            FROM cross_project_search_log
            WHERE searcher_project_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&project_id_str)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter().map(|row| row.into_log()).collect()
    }
}

// ========== Database Row Types ==========

#[derive(sqlx::FromRow)]
struct SettingsRow {
    project_id: String,
    allow_cross_project_search: bool,
    searchable_by_all: bool,
    allowed_searchers: Option<String>,
    excluded_searchers: Option<String>,
    include_features: bool,
    include_conversations: bool,
    include_documents: bool,
    include_checkpoints: bool,
    include_skills: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SettingsRow {
    fn into_settings(self) -> Result<ProjectSearchSettings> {
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;

        let allowed_searchers: Vec<Uuid> = self
            .allowed_searchers
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid allowed_searchers: {}", e)))?
            .unwrap_or_default();

        let excluded_searchers: Vec<Uuid> = self
            .excluded_searchers
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid excluded_searchers: {}", e)))?
            .unwrap_or_default();

        Ok(ProjectSearchSettings {
            project_id,
            allow_cross_project_search: self.allow_cross_project_search,
            searchable_by_all: self.searchable_by_all,
            allowed_searchers,
            excluded_searchers,
            include_features: self.include_features,
            include_conversations: self.include_conversations,
            include_documents: self.include_documents,
            include_checkpoints: self.include_checkpoints,
            include_skills: self.include_skills,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct FeatureResultRow {
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    project_name: String,
    score: f64,
}

impl FeatureResultRow {
    fn into_search_result(self) -> Result<SearchResult> {
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;

        let snippet = self.description.unwrap_or_default();
        let snippet = if snippet.len() > 200 {
            format!("{}...", &snippet[..200])
        } else {
            snippet
        };

        Ok(SearchResult::new(
            self.id,
            SearchEntityType::Feature,
            project_id,
            self.title,
            snippet,
        )
        .with_project_name(self.project_name)
        .with_score(self.score.abs()) // BM25 returns negative scores; lower is better
        .with_created_at(self.created_at))
    }
}

#[derive(sqlx::FromRow)]
struct DocumentResultRow {
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    project_name: String,
    score: f64,
}

impl DocumentResultRow {
    fn into_search_result(self) -> Result<SearchResult> {
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;

        let snippet = self.description.unwrap_or_default();
        let snippet = if snippet.len() > 200 {
            format!("{}...", &snippet[..200])
        } else {
            snippet
        };

        Ok(SearchResult::new(
            self.id,
            SearchEntityType::Document,
            project_id,
            self.title,
            snippet,
        )
        .with_project_name(self.project_name)
        .with_score(self.score.abs())
        .with_created_at(self.created_at))
    }
}

#[derive(sqlx::FromRow)]
struct MessageResultRow {
    id: String,
    project_id: String,
    conv_title: Option<String>,
    content: String,
    created_at: DateTime<Utc>,
    project_name: String,
    score: f64,
}

impl MessageResultRow {
    fn into_search_result(self) -> Result<SearchResult> {
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;

        let title = self
            .conv_title
            .unwrap_or_else(|| "Conversation".to_string());
        let snippet = if self.content.len() > 200 {
            format!("{}...", &self.content[..200])
        } else {
            self.content
        };

        Ok(SearchResult::new(
            self.id,
            SearchEntityType::Message,
            project_id,
            title,
            snippet,
        )
        .with_project_name(self.project_name)
        .with_score(self.score.abs())
        .with_created_at(self.created_at))
    }
}

#[derive(sqlx::FromRow)]
struct SkillResultRow {
    id: String,
    source_project_id: Option<String>,
    name: String,
    description: String,
    created_at: DateTime<Utc>,
    project_name: Option<String>,
    score: f64,
}

impl SkillResultRow {
    fn into_search_result(self) -> Result<SearchResult> {
        let project_id = self
            .source_project_id
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?
            .unwrap_or(Uuid::nil());

        let snippet = if self.description.len() > 200 {
            format!("{}...", &self.description[..200])
        } else {
            self.description
        };

        let mut result = SearchResult::new(
            self.id,
            SearchEntityType::Skill,
            project_id,
            self.name,
            snippet,
        )
        .with_score(self.score.abs())
        .with_created_at(self.created_at);

        if let Some(name) = self.project_name {
            result = result.with_project_name(name);
        }

        Ok(result)
    }
}

#[derive(sqlx::FromRow)]
struct SearchLogRow {
    id: String,
    searcher_project_id: String,
    query: String,
    searched_project_ids: String,
    result_count: i32,
    search_scope: String,
    created_at: DateTime<Utc>,
}

impl SearchLogRow {
    fn into_log(self) -> Result<CrossProjectSearchLog> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| Error::Parse(format!("Invalid log ID: {}", e)))?;
        let searcher_project_id = Uuid::parse_str(&self.searcher_project_id)
            .map_err(|e| Error::Parse(format!("Invalid searcher project ID: {}", e)))?;
        let searched_project_ids: Vec<Uuid> = serde_json::from_str(&self.searched_project_ids)
            .map_err(|e| Error::Parse(format!("Invalid searched_project_ids: {}", e)))?;
        let search_scope_strs: Vec<String> = serde_json::from_str(&self.search_scope)
            .map_err(|e| Error::Parse(format!("Invalid search_scope: {}", e)))?;
        let search_scope: Vec<SearchEntityType> = search_scope_strs
            .iter()
            .filter_map(|s| SearchEntityType::parse(s))
            .collect();

        Ok(CrossProjectSearchLog {
            id,
            searcher_project_id,
            query: self.query,
            searched_project_ids,
            result_count: self.result_count as u32,
            search_scope,
            created_at: self.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    async fn create_test_db() -> SqlitePool {
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");
        db.pool().clone()
    }

    #[tokio::test]
    async fn test_save_and_get_settings() {
        let pool = create_test_db().await;
        let repo = SearchRepository::new(pool.clone());

        // First create a project
        let project_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projects (id, name, framework, repo_url) VALUES (?, 'test', 'rust', '')",
        )
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Save settings
        let settings = ProjectSearchSettings::default_for_project(project_id);
        repo.save_settings(&settings).await.unwrap();

        // Retrieve settings
        let retrieved = repo.get_settings(project_id).await.unwrap().unwrap();
        assert_eq!(retrieved.project_id, project_id);
        assert!(retrieved.allow_cross_project_search);
        assert!(retrieved.searchable_by_all);
    }

    #[tokio::test]
    async fn test_get_or_create_settings() {
        let pool = create_test_db().await;
        let repo = SearchRepository::new(pool.clone());

        // Create a project
        let project_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projects (id, name, framework, repo_url) VALUES (?, 'test', 'rust', '')",
        )
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Get or create (should create)
        let settings = repo.get_or_create_settings(project_id).await.unwrap();
        assert_eq!(settings.project_id, project_id);

        // Get or create again (should get existing)
        let settings2 = repo.get_or_create_settings(project_id).await.unwrap();
        assert_eq!(settings2.project_id, project_id);
    }

    #[tokio::test]
    async fn test_log_search() {
        let pool = create_test_db().await;
        let repo = SearchRepository::new(pool.clone());

        // Create a project
        let project_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projects (id, name, framework, repo_url) VALUES (?, 'test', 'rust', '')",
        )
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Log a search
        let log = CrossProjectSearchLog::new(
            project_id,
            "test query",
            vec![Uuid::new_v4()],
            5,
            vec![SearchEntityType::Feature, SearchEntityType::Document],
        );
        repo.log_search(&log).await.unwrap();

        // Retrieve history
        let history = repo.get_search_history(project_id, 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].query, "test query");
        assert_eq!(history[0].result_count, 5);
    }
}
