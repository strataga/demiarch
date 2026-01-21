//! Search service for orchestrating cross-project search
//!
//! Handles search execution with privacy filtering and audit logging.

use super::entity::{
    CrossProjectSearchLog, ProjectSearchSettings, SearchEntityType, SearchQuery, SearchResult,
    SearchScope,
};
use super::repository::SearchRepository;
use crate::error::{Error, Result};
use sqlx::SqlitePool;
use std::collections::HashSet;
use uuid::Uuid;

/// Service for cross-project search with privacy controls
#[derive(Debug, Clone)]
pub struct SearchService {
    repository: SearchRepository,
}

impl SearchService {
    /// Create a new search service
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            repository: SearchRepository::new(pool),
        }
    }

    /// Get the underlying repository
    pub fn repository(&self) -> &SearchRepository {
        &self.repository
    }

    /// Execute a search query with privacy filtering
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // Determine which projects to search based on scope
        let (searchable_projects, searcher_project_id) = match &query.scope {
            SearchScope::CurrentProject(project_id) => {
                // Single project search - no privacy filtering needed
                (vec![*project_id], None)
            }
            SearchScope::CrossProject {
                from_project,
                target_projects,
            } => {
                // Cross-project search - apply privacy filtering
                let searcher_settings = self
                    .repository
                    .get_or_create_settings(*from_project)
                    .await?;

                if !searcher_settings.allow_cross_project_search {
                    return Err(Error::Validation(
                        "Cross-project search is disabled for this project".to_string(),
                    ));
                }

                let projects = self
                    .get_accessible_projects(*from_project, target_projects.as_deref())
                    .await?;

                (projects, Some(*from_project))
            }
            SearchScope::Global => {
                // Global search - get all active projects
                let all_projects = self.repository.get_searchable_projects(Uuid::nil()).await?;
                (all_projects, None)
            }
        };

        if searchable_projects.is_empty() {
            return Ok(Vec::new());
        }

        // Execute searches for each entity type
        let mut all_results = Vec::new();
        let entity_types = self
            .get_searchable_entity_types(query, &searchable_projects, searcher_project_id)
            .await?;

        // Distribute limit across entity types for balanced results
        let limit_per_type = (query.limit / entity_types.len().max(1) as u32).max(5);

        for entity_type in &entity_types {
            let results = match entity_type {
                SearchEntityType::Feature => {
                    let feature_projects = self
                        .filter_projects_by_entity_type(
                            &searchable_projects,
                            *entity_type,
                            searcher_project_id,
                        )
                        .await?;
                    self.repository
                        .search_features(&query.query, &feature_projects, limit_per_type, 0)
                        .await?
                }
                SearchEntityType::Document => {
                    let doc_projects = self
                        .filter_projects_by_entity_type(
                            &searchable_projects,
                            *entity_type,
                            searcher_project_id,
                        )
                        .await?;
                    self.repository
                        .search_documents(&query.query, &doc_projects, limit_per_type, 0)
                        .await?
                }
                SearchEntityType::Message => {
                    let msg_projects = self
                        .filter_projects_by_entity_type(
                            &searchable_projects,
                            *entity_type,
                            searcher_project_id,
                        )
                        .await?;
                    self.repository
                        .search_messages(&query.query, &msg_projects, limit_per_type, 0)
                        .await?
                }
                SearchEntityType::Skill => {
                    let skill_projects = self
                        .filter_projects_by_entity_type(
                            &searchable_projects,
                            *entity_type,
                            searcher_project_id,
                        )
                        .await?;
                    self.repository
                        .search_skills(&query.query, &skill_projects, limit_per_type, 0)
                        .await?
                }
                SearchEntityType::Checkpoint => {
                    // Checkpoints not yet indexed for full-text search
                    Vec::new()
                }
            };
            all_results.extend(results);
        }

        // Sort by score (lower BM25 is better, so we sorted by score ascending)
        all_results.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply final limit
        all_results.truncate(query.limit as usize);

        // Log cross-project search for audit
        if let Some(searcher_id) = searcher_project_id {
            let log = CrossProjectSearchLog::new(
                searcher_id,
                &query.query,
                searchable_projects.clone(),
                all_results.len() as u32,
                entity_types,
            );
            // Log asynchronously (don't fail search if logging fails)
            let _ = self.repository.log_search(&log).await;
        }

        Ok(all_results)
    }

    /// Get search settings for a project
    pub async fn get_settings(&self, project_id: Uuid) -> Result<ProjectSearchSettings> {
        self.repository.get_or_create_settings(project_id).await
    }

    /// Update search settings for a project
    pub async fn update_settings(&self, settings: &ProjectSearchSettings) -> Result<()> {
        self.repository.save_settings(settings).await
    }

    /// Enable cross-project search for a project
    pub async fn enable_cross_project_search(&self, project_id: Uuid) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;
        settings.allow_cross_project_search = true;
        self.repository.save_settings(&settings).await
    }

    /// Disable cross-project search for a project (opt-out)
    pub async fn disable_cross_project_search(&self, project_id: Uuid) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;
        settings.allow_cross_project_search = false;
        self.repository.save_settings(&settings).await
    }

    /// Make a project searchable by all other projects
    pub async fn make_searchable_by_all(&self, project_id: Uuid) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;
        settings.searchable_by_all = true;
        self.repository.save_settings(&settings).await
    }

    /// Make a project private (not searchable by other projects)
    pub async fn make_private(&self, project_id: Uuid) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;
        settings.searchable_by_all = false;
        settings.allowed_searchers.clear();
        self.repository.save_settings(&settings).await
    }

    /// Allow a specific project to search this one
    pub async fn allow_searcher(&self, project_id: Uuid, searcher_id: Uuid) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;
        if !settings.allowed_searchers.contains(&searcher_id) {
            settings.allowed_searchers.push(searcher_id);
        }
        // Remove from excluded if present
        settings.excluded_searchers.retain(|id| *id != searcher_id);
        self.repository.save_settings(&settings).await
    }

    /// Block a specific project from searching this one
    pub async fn block_searcher(&self, project_id: Uuid, searcher_id: Uuid) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;
        if !settings.excluded_searchers.contains(&searcher_id) {
            settings.excluded_searchers.push(searcher_id);
        }
        // Remove from allowed if present
        settings.allowed_searchers.retain(|id| *id != searcher_id);
        self.repository.save_settings(&settings).await
    }

    /// Configure which entity types are searchable for a project
    pub async fn set_searchable_entity_types(
        &self,
        project_id: Uuid,
        types: &[SearchEntityType],
    ) -> Result<()> {
        let mut settings = self.repository.get_or_create_settings(project_id).await?;

        let types_set: HashSet<_> = types.iter().collect();
        settings.include_features = types_set.contains(&SearchEntityType::Feature);
        settings.include_documents = types_set.contains(&SearchEntityType::Document);
        settings.include_conversations = types_set.contains(&SearchEntityType::Message);
        settings.include_skills = types_set.contains(&SearchEntityType::Skill);
        settings.include_checkpoints = types_set.contains(&SearchEntityType::Checkpoint);

        self.repository.save_settings(&settings).await
    }

    /// Get search history for a project
    pub async fn get_search_history(
        &self,
        project_id: Uuid,
        limit: u32,
    ) -> Result<Vec<CrossProjectSearchLog>> {
        self.repository.get_search_history(project_id, limit).await
    }

    /// Get all projects accessible to a searcher
    pub async fn get_accessible_projects(
        &self,
        searcher_project_id: Uuid,
        target_projects: Option<&[Uuid]>,
    ) -> Result<Vec<Uuid>> {
        // Get all potentially searchable projects
        let all_searchable = self
            .repository
            .get_searchable_projects(searcher_project_id)
            .await?;

        // If specific targets requested, filter to those
        if let Some(targets) = target_projects {
            let target_set: HashSet<_> = targets.iter().collect();
            Ok(all_searchable
                .into_iter()
                .filter(|id| target_set.contains(id))
                .collect())
        } else {
            Ok(all_searchable)
        }
    }

    // ========== Private Helper Methods ==========

    /// Get entity types to search based on query and project settings
    async fn get_searchable_entity_types(
        &self,
        query: &SearchQuery,
        _project_ids: &[Uuid],
        _searcher_project_id: Option<Uuid>,
    ) -> Result<Vec<SearchEntityType>> {
        // If query specifies entity types, use those
        if !query.entity_types.is_empty() {
            return Ok(query.entity_types.clone());
        }

        // Otherwise, return all types (will be filtered per-project later)
        Ok(vec![
            SearchEntityType::Feature,
            SearchEntityType::Document,
            SearchEntityType::Message,
            SearchEntityType::Skill,
        ])
    }

    /// Filter projects by what entity types they allow to be searched
    async fn filter_projects_by_entity_type(
        &self,
        project_ids: &[Uuid],
        entity_type: SearchEntityType,
        searcher_project_id: Option<Uuid>,
    ) -> Result<Vec<Uuid>> {
        let mut filtered = Vec::new();

        for &project_id in project_ids {
            // For current project search, allow all entity types
            if searcher_project_id.is_none() || searcher_project_id == Some(project_id) {
                filtered.push(project_id);
                continue;
            }

            // For cross-project search, check settings
            if let Some(settings) = self.repository.get_settings(project_id).await? {
                if settings.can_search_entity_type(entity_type) {
                    filtered.push(project_id);
                }
            } else {
                // No settings = use defaults (features, documents, skills enabled)
                let default_settings = ProjectSearchSettings::default_for_project(project_id);
                if default_settings.can_search_entity_type(entity_type) {
                    filtered.push(project_id);
                }
            }
        }

        Ok(filtered)
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

    async fn create_test_project(pool: &SqlitePool, name: &str) -> Uuid {
        let project_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projects (id, name, framework, repo_url) VALUES (?, ?, 'rust', '')",
        )
        .bind(project_id.to_string())
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
        project_id
    }

    #[tokio::test]
    async fn test_get_and_update_settings() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "test-project").await;

        // Get default settings
        let settings = service.get_settings(project_id).await.unwrap();
        assert!(settings.allow_cross_project_search);
        assert!(settings.searchable_by_all);

        // Update settings
        let mut updated = settings.clone();
        updated.include_conversations = true;
        service.update_settings(&updated).await.unwrap();

        // Verify update
        let settings = service.get_settings(project_id).await.unwrap();
        assert!(settings.include_conversations);
    }

    #[tokio::test]
    async fn test_disable_cross_project_search() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "test-project").await;

        // Disable cross-project search
        service
            .disable_cross_project_search(project_id)
            .await
            .unwrap();

        // Verify
        let settings = service.get_settings(project_id).await.unwrap();
        assert!(!settings.allow_cross_project_search);
    }

    #[tokio::test]
    async fn test_make_private() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "test-project").await;

        // Make private
        service.make_private(project_id).await.unwrap();

        // Verify
        let settings = service.get_settings(project_id).await.unwrap();
        assert!(!settings.searchable_by_all);
    }

    #[tokio::test]
    async fn test_allow_and_block_searcher() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "project-a").await;
        let searcher_id = create_test_project(&pool, "project-b").await;

        // Make private first
        service.make_private(project_id).await.unwrap();

        // Allow specific searcher
        service
            .allow_searcher(project_id, searcher_id)
            .await
            .unwrap();

        let settings = service.get_settings(project_id).await.unwrap();
        assert!(settings.allowed_searchers.contains(&searcher_id));
        assert!(settings.can_be_searched_by(searcher_id));

        // Block the searcher
        service
            .block_searcher(project_id, searcher_id)
            .await
            .unwrap();

        let settings = service.get_settings(project_id).await.unwrap();
        assert!(settings.excluded_searchers.contains(&searcher_id));
        assert!(!settings.allowed_searchers.contains(&searcher_id));
        assert!(!settings.can_be_searched_by(searcher_id));
    }

    #[tokio::test]
    async fn test_set_searchable_entity_types() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "test-project").await;

        // Set only features and skills searchable
        service
            .set_searchable_entity_types(
                project_id,
                &[SearchEntityType::Feature, SearchEntityType::Skill],
            )
            .await
            .unwrap();

        let settings = service.get_settings(project_id).await.unwrap();
        assert!(settings.include_features);
        assert!(settings.include_skills);
        assert!(!settings.include_documents);
        assert!(!settings.include_conversations);
    }

    #[tokio::test]
    async fn test_search_disabled_project() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "test-project").await;

        // Disable cross-project search
        service
            .disable_cross_project_search(project_id)
            .await
            .unwrap();

        // Try to search across projects
        let query = SearchQuery::new("test").with_scope(SearchScope::cross_project(project_id));

        let result = service.search(&query).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_current_project_search_always_allowed() {
        let pool = create_test_db().await;
        let service = SearchService::new(pool.clone());

        let project_id = create_test_project(&pool, "test-project").await;

        // Disable cross-project search
        service
            .disable_cross_project_search(project_id)
            .await
            .unwrap();

        // Current project search should still work
        let query = SearchQuery::new("test").with_scope(SearchScope::current_project(project_id));

        let result = service.search(&query).await;
        assert!(result.is_ok());
    }
}
