//! Repository trait for search persistence
//!
//! This module defines the trait for search storage operations.
//! The trait abstracts over different storage backends (SQLite, etc.).

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;

use super::entity::{CrossProjectSearchLog, ProjectSearchSettings, SearchResult};

/// Repository trait for search persistence
///
/// Provides operations for search settings, full-text search, and audit logging.
#[async_trait]
pub trait SearchRepositoryTrait: Send + Sync {
    // ========== Project Search Settings ==========

    /// Get search settings for a project
    async fn get_settings(&self, project_id: Uuid) -> Result<Option<ProjectSearchSettings>>;

    /// Get or create default settings for a project
    async fn get_or_create_settings(&self, project_id: Uuid) -> Result<ProjectSearchSettings>;

    /// Save search settings for a project
    async fn save_settings(&self, settings: &ProjectSearchSettings) -> Result<()>;

    /// Get all projects that can be searched by a given project
    async fn get_searchable_projects(&self, searcher_project_id: Uuid) -> Result<Vec<Uuid>>;

    // ========== Full-Text Search ==========

    /// Search features across specified projects
    async fn search_features(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>>;

    /// Search documents across specified projects
    async fn search_documents(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>>;

    /// Search messages (conversations) across specified projects
    async fn search_messages(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>>;

    /// Search learned skills (global or filtered by source project)
    async fn search_skills(
        &self,
        query: &str,
        project_ids: &[Uuid],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchResult>>;

    // ========== Search Audit Log ==========

    /// Log a cross-project search for audit trail
    async fn log_search(&self, log: &CrossProjectSearchLog) -> Result<()>;

    /// Get search history for a project
    async fn get_search_history(
        &self,
        project_id: Uuid,
        limit: u32,
    ) -> Result<Vec<CrossProjectSearchLog>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify trait is object-safe
    fn _assert_object_safe(_: &dyn SearchRepositoryTrait) {}
}
