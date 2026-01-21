//! Search entity and related types
//!
//! Defines the core types for cross-project search functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Types of entities that can be searched
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchEntityType {
    /// Features/tasks
    Feature,
    /// Documents (PRDs, architecture docs, etc.)
    Document,
    /// Conversation messages
    Message,
    /// Learned skills
    Skill,
    /// Checkpoints
    Checkpoint,
}

impl SearchEntityType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Document => "document",
            Self::Message => "message",
            Self::Skill => "skill",
            Self::Checkpoint => "checkpoint",
        }
    }

    /// Create from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "feature" => Some(Self::Feature),
            "document" => Some(Self::Document),
            "message" => Some(Self::Message),
            "skill" => Some(Self::Skill),
            "checkpoint" => Some(Self::Checkpoint),
            _ => None,
        }
    }

    /// Get all entity types
    pub fn all() -> Vec<Self> {
        vec![
            Self::Feature,
            Self::Document,
            Self::Message,
            Self::Skill,
            Self::Checkpoint,
        ]
    }
}

impl fmt::Display for SearchEntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Search scope defining where to search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchScope {
    /// Search only within the current project
    CurrentProject(Uuid),

    /// Search across projects with privacy controls
    CrossProject {
        /// The project initiating the search
        from_project: Uuid,
        /// Optional list of specific projects to search (if empty, search all accessible)
        target_projects: Option<Vec<Uuid>>,
    },

    /// Search globally without project context (for system-level queries)
    Global,
}

impl SearchScope {
    /// Create a current-project search scope
    pub fn current_project(project_id: Uuid) -> Self {
        Self::CurrentProject(project_id)
    }

    /// Create a cross-project search scope
    pub fn cross_project(from_project: Uuid) -> Self {
        Self::CrossProject {
            from_project,
            target_projects: None,
        }
    }

    /// Create a cross-project search scope with specific targets
    pub fn cross_project_with_targets(from_project: Uuid, targets: Vec<Uuid>) -> Self {
        Self::CrossProject {
            from_project,
            target_projects: Some(targets),
        }
    }

    /// Check if this is a cross-project search
    pub fn is_cross_project(&self) -> bool {
        matches!(self, Self::CrossProject { .. } | Self::Global)
    }
}

/// A search query with all parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search text/query
    pub query: String,

    /// Search scope (single project or cross-project)
    pub scope: SearchScope,

    /// Entity types to search (if empty, search all)
    pub entity_types: Vec<SearchEntityType>,

    /// Maximum number of results to return
    pub limit: u32,

    /// Offset for pagination
    pub offset: u32,
}

impl SearchQuery {
    /// Create a new search query with default settings
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            scope: SearchScope::Global,
            entity_types: Vec::new(), // Search all types by default
            limit: 50,
            offset: 0,
        }
    }

    /// Set the search scope
    pub fn with_scope(mut self, scope: SearchScope) -> Self {
        self.scope = scope;
        self
    }

    /// Set the entity types to search
    pub fn with_entity_types(mut self, types: Vec<SearchEntityType>) -> Self {
        self.entity_types = types;
        self
    }

    /// Add a single entity type to search
    pub fn with_entity_type(mut self, entity_type: SearchEntityType) -> Self {
        if !self.entity_types.contains(&entity_type) {
            self.entity_types.push(entity_type);
        }
        self
    }

    /// Set the result limit
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Set the offset for pagination
    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    /// Check if a specific entity type should be searched
    pub fn should_search(&self, entity_type: SearchEntityType) -> bool {
        self.entity_types.is_empty() || self.entity_types.contains(&entity_type)
    }
}

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier of the matched entity
    pub entity_id: String,

    /// Type of entity that matched
    pub entity_type: SearchEntityType,

    /// Project that contains this entity
    pub project_id: Uuid,

    /// Project name for display
    pub project_name: Option<String>,

    /// Title or name of the matched entity
    pub title: String,

    /// Snippet of matching content with highlights
    pub snippet: String,

    /// Relevance score (higher is better)
    pub score: f64,

    /// When the entity was created
    pub created_at: DateTime<Utc>,

    /// Additional metadata specific to entity type
    pub metadata: Option<serde_json::Value>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(
        entity_id: impl Into<String>,
        entity_type: SearchEntityType,
        project_id: Uuid,
        title: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        Self {
            entity_id: entity_id.into(),
            entity_type,
            project_id,
            project_name: None,
            title: title.into(),
            snippet: snippet.into(),
            score: 0.0,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// Set the project name
    pub fn with_project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = Some(name.into());
        self
    }

    /// Set the relevance score
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    /// Set the created_at timestamp
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Set additional metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Project search settings controlling cross-project search privacy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSearchSettings {
    /// Project ID these settings apply to
    pub project_id: Uuid,

    /// Whether this project can search across other projects
    pub allow_cross_project_search: bool,

    /// Whether this project can be searched by other projects
    pub searchable_by_all: bool,

    /// Specific projects that are allowed to search this one (if not searchable_by_all)
    pub allowed_searchers: Vec<Uuid>,

    /// Projects explicitly blocked from searching this one
    pub excluded_searchers: Vec<Uuid>,

    /// Include features in search
    pub include_features: bool,

    /// Include conversations in search
    pub include_conversations: bool,

    /// Include documents in search
    pub include_documents: bool,

    /// Include checkpoints in search
    pub include_checkpoints: bool,

    /// Include skills in search
    pub include_skills: bool,

    /// When settings were created
    pub created_at: DateTime<Utc>,

    /// When settings were last updated
    pub updated_at: DateTime<Utc>,
}

impl ProjectSearchSettings {
    /// Create default settings for a project (opt-in by default)
    pub fn default_for_project(project_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            project_id,
            allow_cross_project_search: true,
            searchable_by_all: true,
            allowed_searchers: Vec::new(),
            excluded_searchers: Vec::new(),
            include_features: true,
            include_conversations: false, // Off by default for privacy
            include_documents: true,
            include_checkpoints: false, // Off by default
            include_skills: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create private settings (opt-out of cross-project search)
    pub fn private(project_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            project_id,
            allow_cross_project_search: false,
            searchable_by_all: false,
            allowed_searchers: Vec::new(),
            excluded_searchers: Vec::new(),
            include_features: false,
            include_conversations: false,
            include_documents: false,
            include_checkpoints: false,
            include_skills: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if a specific project can search this one
    pub fn can_be_searched_by(&self, searcher_project_id: Uuid) -> bool {
        // Can't search yourself (use CurrentProject scope instead)
        if searcher_project_id == self.project_id {
            return false;
        }

        // Check if explicitly excluded
        if self.excluded_searchers.contains(&searcher_project_id) {
            return false;
        }

        // If searchable by all, allow
        if self.searchable_by_all {
            return true;
        }

        // Otherwise, must be in allowed list
        self.allowed_searchers.contains(&searcher_project_id)
    }

    /// Check if a specific entity type can be searched in this project
    pub fn can_search_entity_type(&self, entity_type: SearchEntityType) -> bool {
        match entity_type {
            SearchEntityType::Feature => self.include_features,
            SearchEntityType::Document => self.include_documents,
            SearchEntityType::Message => self.include_conversations,
            SearchEntityType::Skill => self.include_skills,
            SearchEntityType::Checkpoint => self.include_checkpoints,
        }
    }

    /// Get the list of searchable entity types
    pub fn searchable_entity_types(&self) -> Vec<SearchEntityType> {
        let mut types = Vec::new();
        if self.include_features {
            types.push(SearchEntityType::Feature);
        }
        if self.include_documents {
            types.push(SearchEntityType::Document);
        }
        if self.include_conversations {
            types.push(SearchEntityType::Message);
        }
        if self.include_skills {
            types.push(SearchEntityType::Skill);
        }
        if self.include_checkpoints {
            types.push(SearchEntityType::Checkpoint);
        }
        types
    }
}

/// Log entry for cross-project searches (audit trail)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossProjectSearchLog {
    /// Unique log entry ID
    pub id: Uuid,

    /// Project that initiated the search
    pub searcher_project_id: Uuid,

    /// The search query
    pub query: String,

    /// Projects that were searched
    pub searched_project_ids: Vec<Uuid>,

    /// Number of results returned
    pub result_count: u32,

    /// Entity types that were searched
    pub search_scope: Vec<SearchEntityType>,

    /// When the search occurred
    pub created_at: DateTime<Utc>,
}

impl CrossProjectSearchLog {
    /// Create a new search log entry
    pub fn new(
        searcher_project_id: Uuid,
        query: impl Into<String>,
        searched_project_ids: Vec<Uuid>,
        result_count: u32,
        search_scope: Vec<SearchEntityType>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            searcher_project_id,
            query: query.into(),
            searched_project_ids,
            result_count,
            search_scope,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_entity_type_conversion() {
        assert_eq!(SearchEntityType::Feature.as_str(), "feature");
        assert_eq!(
            SearchEntityType::from_str("feature"),
            Some(SearchEntityType::Feature)
        );
        assert_eq!(SearchEntityType::from_str("invalid"), None);
    }

    #[test]
    fn test_search_query_builder() {
        let project_id = Uuid::new_v4();
        let query = SearchQuery::new("test query")
            .with_scope(SearchScope::current_project(project_id))
            .with_entity_type(SearchEntityType::Feature)
            .with_limit(10);

        assert_eq!(query.query, "test query");
        assert_eq!(query.limit, 10);
        assert!(query.should_search(SearchEntityType::Feature));
        assert!(!query.should_search(SearchEntityType::Document));
    }

    #[test]
    fn test_search_settings_permissions() {
        let project_id = Uuid::new_v4();
        let searcher_a = Uuid::new_v4();
        let searcher_b = Uuid::new_v4();
        let searcher_c = Uuid::new_v4();

        // Test default (searchable by all)
        let settings = ProjectSearchSettings::default_for_project(project_id);
        assert!(settings.can_be_searched_by(searcher_a));
        assert!(settings.can_be_searched_by(searcher_b));

        // Test with exclusion
        let mut settings = ProjectSearchSettings::default_for_project(project_id);
        settings.excluded_searchers.push(searcher_a);
        assert!(!settings.can_be_searched_by(searcher_a));
        assert!(settings.can_be_searched_by(searcher_b));

        // Test with allowlist only
        let mut settings = ProjectSearchSettings::default_for_project(project_id);
        settings.searchable_by_all = false;
        settings.allowed_searchers.push(searcher_b);
        assert!(!settings.can_be_searched_by(searcher_a));
        assert!(settings.can_be_searched_by(searcher_b));
        assert!(!settings.can_be_searched_by(searcher_c));

        // Test private settings
        let settings = ProjectSearchSettings::private(project_id);
        assert!(!settings.can_be_searched_by(searcher_a));
    }

    #[test]
    fn test_search_settings_entity_types() {
        let project_id = Uuid::new_v4();
        let settings = ProjectSearchSettings::default_for_project(project_id);

        // Default: features, documents, skills enabled; conversations, checkpoints disabled
        assert!(settings.can_search_entity_type(SearchEntityType::Feature));
        assert!(settings.can_search_entity_type(SearchEntityType::Document));
        assert!(settings.can_search_entity_type(SearchEntityType::Skill));
        assert!(!settings.can_search_entity_type(SearchEntityType::Message));
        assert!(!settings.can_search_entity_type(SearchEntityType::Checkpoint));

        let types = settings.searchable_entity_types();
        assert_eq!(types.len(), 3);
    }

    #[test]
    fn test_cannot_search_self() {
        let project_id = Uuid::new_v4();
        let settings = ProjectSearchSettings::default_for_project(project_id);

        // Can't search yourself via cross-project
        assert!(!settings.can_be_searched_by(project_id));
    }
}
