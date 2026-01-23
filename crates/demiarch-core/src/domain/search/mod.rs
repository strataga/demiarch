//! Search domain module
//!
//! Provides cross-project search with privacy controls.
//!
//! # Architecture
//!
//! - **Entities**: `SearchResult`, `SearchSettings`, `SearchQuery`
//! - **Repository**: `SearchRepository` for database operations
//! - **Service**: `SearchService` for search orchestration with privacy filtering
//!
//! # Features
//!
//! - Cross-project full-text search using SQLite FTS5
//! - Privacy controls: opt-in/out per project
//! - Granular search scope (features, documents, conversations, skills)
//! - Audit trail for cross-project searches
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::search::{SearchService, SearchQuery, SearchScope};
//! use sqlx::SqlitePool;
//!
//! // Create service
//! let service = SearchService::new(pool.clone());
//!
//! // Search within current project only
//! let query = SearchQuery::new("authentication")
//!     .with_scope(SearchScope::CurrentProject(project_id));
//! let results = service.search(&query).await?;
//!
//! // Search across all accessible projects
//! let query = SearchQuery::new("authentication")
//!     .with_scope(SearchScope::CrossProject { from_project: project_id });
//! let results = service.search(&query).await?;
//! ```

pub mod entity;
pub mod event;
pub mod repository;
pub mod repository_trait;
pub mod service;
pub mod specification;

// Re-export main types
pub use entity::{
    CrossProjectSearchLog, ProjectSearchSettings, SearchEntityType, SearchQuery, SearchResult,
    SearchScope,
};
pub use event::{SearchEvent, SearchEventType};
pub use repository::SearchRepository;
pub use repository_trait::SearchRepositoryTrait;
pub use service::SearchService;
pub use specification::{
    EntityTypeSpec, MinRelevanceSpec, PrivacyAllowedSpec, ScopeSpec, SearchSpecBuilder,
};
