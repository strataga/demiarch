//! Feature management commands
//!
//! Provides CRUD operations for project features.

use crate::storage::Database;
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// Feature status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureStatus {
    #[default]
    Backlog,
    Todo,
    InProgress,
    Review,
    Done,
}

impl FeatureStatus {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureStatus::Backlog => "backlog",
            FeatureStatus::Todo => "todo",
            FeatureStatus::InProgress => "in_progress",
            FeatureStatus::Review => "review",
            FeatureStatus::Done => "done",
        }
    }

    /// Parse from database string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "backlog" => Some(FeatureStatus::Backlog),
            "todo" => Some(FeatureStatus::Todo),
            "in_progress" => Some(FeatureStatus::InProgress),
            "review" => Some(FeatureStatus::Review),
            "done" => Some(FeatureStatus::Done),
            _ => None,
        }
    }
}

/// A project feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// Unique feature identifier
    pub id: String,
    /// Associated project ID
    pub project_id: String,
    /// Feature title
    pub title: String,
    /// Feature description
    pub description: Option<String>,
    /// Acceptance criteria for the feature
    pub acceptance_criteria: Option<String>,
    /// Labels/tags for categorization (stored as JSON array)
    pub labels: Option<Vec<String>>,
    /// Phase ID (for grouping features)
    pub phase_id: Option<String>,
    /// Feature status
    pub status: FeatureStatus,
    /// Priority (1-5, where 1 is highest)
    pub priority: i32,
    /// When the feature was created
    pub created_at: DateTime<Utc>,
    /// When the feature was last updated
    pub updated_at: DateTime<Utc>,
}

impl Feature {
    /// Create a new feature
    pub fn new(project_id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            title: title.into(),
            description: None,
            acceptance_criteria: None,
            labels: None,
            phase_id: None,
            status: FeatureStatus::Backlog,
            priority: 3, // Default to medium priority (1-5 scale)
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set acceptance criteria
    pub fn with_acceptance_criteria(mut self, criteria: impl Into<String>) -> Self {
        self.acceptance_criteria = Some(criteria.into());
        self
    }

    /// Set labels
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Set the phase ID
    pub fn with_phase(mut self, phase_id: impl Into<String>) -> Self {
        self.phase_id = Some(phase_id.into());
        self
    }

    /// Set the priority (1-5, where 1 is highest)
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority.clamp(1, 5);
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: FeatureStatus) -> Self {
        self.status = status;
        self
    }
}

/// Feature repository for database operations
pub struct FeatureRepository<'a> {
    db: &'a Database,
}

impl<'a> FeatureRepository<'a> {
    /// Create a new feature repository
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new feature in the database
    pub async fn create(&self, feature: &Feature) -> Result<()> {
        let labels_json = feature
            .labels
            .as_ref()
            .map(|l| serde_json::to_string(l).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO features (id, project_id, title, description, acceptance_criteria, labels, phase_id, status, priority, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&feature.id)
        .bind(&feature.project_id)
        .bind(&feature.title)
        .bind(&feature.description)
        .bind(&feature.acceptance_criteria)
        .bind(&labels_json)
        .bind(&feature.phase_id)
        .bind(feature.status.as_str())
        .bind(feature.priority)
        .bind(feature.created_at)
        .bind(feature.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get a feature by ID
    pub async fn get(&self, id: &str) -> Result<Option<Feature>> {
        let row = sqlx::query(
            "SELECT id, project_id, title, description, acceptance_criteria, labels, phase_id, status, priority, created_at, updated_at FROM features WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_feature(r)))
    }

    /// List features for a project
    pub async fn list_by_project(
        &self,
        project_id: &str,
        status: Option<FeatureStatus>,
    ) -> Result<Vec<Feature>> {
        let rows = if let Some(status) = status {
            sqlx::query(
                "SELECT id, project_id, title, description, acceptance_criteria, labels, phase_id, status, priority, created_at, updated_at FROM features WHERE project_id = ? AND status = ? ORDER BY priority, created_at",
            )
            .bind(project_id)
            .bind(status.as_str())
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query(
                "SELECT id, project_id, title, description, acceptance_criteria, labels, phase_id, status, priority, created_at, updated_at FROM features WHERE project_id = ? ORDER BY priority, created_at",
            )
            .bind(project_id)
            .fetch_all(self.db.pool())
            .await?
        };

        Ok(rows.into_iter().map(|r| self.row_to_feature(r)).collect())
    }

    /// List features for a phase
    pub async fn list_by_phase(&self, phase_id: &str) -> Result<Vec<Feature>> {
        let rows = sqlx::query(
            "SELECT id, project_id, title, description, acceptance_criteria, labels, phase_id, status, priority, created_at, updated_at FROM features WHERE phase_id = ? ORDER BY priority, created_at",
        )
        .bind(phase_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|r| self.row_to_feature(r)).collect())
    }

    /// Update a feature
    pub async fn update(&self, feature: &Feature) -> Result<()> {
        let labels_json = feature
            .labels
            .as_ref()
            .map(|l| serde_json::to_string(l).unwrap_or_default());

        sqlx::query(
            r#"
            UPDATE features
            SET title = ?, description = ?, acceptance_criteria = ?, labels = ?, phase_id = ?, status = ?, priority = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&feature.title)
        .bind(&feature.description)
        .bind(&feature.acceptance_criteria)
        .bind(&labels_json)
        .bind(&feature.phase_id)
        .bind(feature.status.as_str())
        .bind(feature.priority)
        .bind(Utc::now())
        .bind(&feature.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Move feature to a different phase
    pub async fn move_to_phase(&self, feature_id: &str, phase_id: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE features SET phase_id = ?, updated_at = ? WHERE id = ?")
            .bind(phase_id)
            .bind(Utc::now())
            .bind(feature_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Update feature status
    pub async fn update_status(&self, id: &str, status: FeatureStatus) -> Result<()> {
        sqlx::query("UPDATE features SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Delete a feature
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM features WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Convert a database row to a Feature
    fn row_to_feature(&self, row: sqlx::sqlite::SqliteRow) -> Feature {
        let labels_str: Option<String> = row.get("labels");
        let labels = labels_str
            .and_then(|s| serde_json::from_str(&s).ok());

        Feature {
            id: row.get("id"),
            project_id: row.get("project_id"),
            title: row.get("title"),
            description: row.get("description"),
            acceptance_criteria: row.get("acceptance_criteria"),
            labels,
            phase_id: row.get("phase_id"),
            status: FeatureStatus::parse(row.get("status")).unwrap_or_default(),
            priority: row.get("priority"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

// ============================================================================
// Public API functions
// ============================================================================

/// Create a new feature
pub async fn create(_project_id: &str, _title: &str, _phase_id: Option<&str>) -> Result<String> {
    Ok("feature-placeholder-id".to_string())
}

/// Create a new feature with database
pub async fn create_with_db(
    db: &Database,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    phase_id: Option<&str>,
) -> Result<Feature> {
    let repo = FeatureRepository::new(db);

    let mut feature = Feature::new(project_id, title);
    if let Some(desc) = description {
        feature = feature.with_description(desc);
    }
    if let Some(phase) = phase_id {
        feature = feature.with_phase(phase);
    }

    repo.create(&feature).await?;
    Ok(feature)
}

/// List features
pub async fn list(_project_id: &str, _status: Option<&str>) -> Result<Vec<String>> {
    Ok(Vec::new())
}

/// List features with database
pub async fn list_with_db(
    db: &Database,
    project_id: &str,
    status: Option<FeatureStatus>,
) -> Result<Vec<Feature>> {
    let repo = FeatureRepository::new(db);
    repo.list_by_project(project_id, status).await
}

/// Update feature
pub async fn update(_id: &str, _status: Option<&str>, _priority: Option<i32>) -> Result<()> {
    Ok(())
}

/// Update feature with database
pub async fn update_with_db(
    db: &Database,
    id: &str,
    status: Option<FeatureStatus>,
    priority: Option<i32>,
) -> Result<()> {
    let repo = FeatureRepository::new(db);

    let mut feature = repo
        .get(id)
        .await?
        .ok_or_else(|| crate::Error::NotFound(format!("Feature not found: {}", id)))?;

    if let Some(s) = status {
        feature.status = s;
    }
    if let Some(p) = priority {
        feature.priority = p;
    }

    repo.update(&feature).await
}

/// Delete feature
pub async fn delete(_id: &str) -> Result<()> {
    Ok(())
}

/// Delete feature with database
pub async fn delete_with_db(db: &Database, id: &str) -> Result<()> {
    let repo = FeatureRepository::new(db);

    // Check if feature exists
    if repo.get(id).await?.is_none() {
        return Err(crate::Error::NotFound(format!("Feature not found: {}", id)));
    }

    repo.delete(id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::project::{Project, ProjectRepository};

    #[test]
    fn test_feature_status_parse() {
        assert_eq!(FeatureStatus::parse("backlog"), Some(FeatureStatus::Backlog));
        assert_eq!(FeatureStatus::parse("todo"), Some(FeatureStatus::Todo));
        assert_eq!(FeatureStatus::parse("in_progress"), Some(FeatureStatus::InProgress));
        assert_eq!(FeatureStatus::parse("review"), Some(FeatureStatus::Review));
        assert_eq!(FeatureStatus::parse("done"), Some(FeatureStatus::Done));
        assert_eq!(FeatureStatus::parse("invalid"), None);
    }

    #[test]
    fn test_feature_new() {
        let feature = Feature::new("proj-123", "Test Feature");

        assert!(!feature.id.is_empty());
        assert_eq!(feature.project_id, "proj-123");
        assert_eq!(feature.title, "Test Feature");
        assert_eq!(feature.status, FeatureStatus::Backlog);
        assert_eq!(feature.priority, 3); // Default medium priority
    }

    #[test]
    fn test_feature_builders() {
        let feature = Feature::new("proj-123", "Test Feature")
            .with_description("A test feature")
            .with_acceptance_criteria("Given X When Y Then Z")
            .with_labels(vec!["backend".to_string(), "api".to_string()])
            .with_phase("phase-1")
            .with_priority(1);

        assert_eq!(feature.description, Some("A test feature".to_string()));
        assert_eq!(feature.acceptance_criteria, Some("Given X When Y Then Z".to_string()));
        assert_eq!(feature.labels, Some(vec!["backend".to_string(), "api".to_string()]));
        assert_eq!(feature.phase_id, Some("phase-1".to_string()));
        assert_eq!(feature.priority, 1);
    }

    #[test]
    fn test_priority_clamping() {
        let feature = Feature::new("proj-123", "Test Feature").with_priority(10);
        assert_eq!(feature.priority, 5); // Clamped to max

        let feature = Feature::new("proj-123", "Test Feature").with_priority(0);
        assert_eq!(feature.priority, 1); // Clamped to min
    }

    #[tokio::test]
    async fn test_feature_repository_crud() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        // Create a project first
        let project = Project::new("test-project", "rust", "");
        let project_repo = ProjectRepository::new(&db);
        project_repo.create(&project).await.unwrap();

        let repo = FeatureRepository::new(&db);

        // Create
        let feature = Feature::new(&project.id, "Test Feature")
            .with_description("A test feature");
        repo.create(&feature).await.unwrap();

        // Read
        let retrieved = repo.get(&feature.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Test Feature");

        // List
        let features = repo.list_by_project(&project.id, None).await.unwrap();
        assert_eq!(features.len(), 1);

        // Update status
        repo.update_status(&feature.id, FeatureStatus::InProgress).await.unwrap();
        let updated = repo.get(&feature.id).await.unwrap().unwrap();
        assert_eq!(updated.status, FeatureStatus::InProgress);

        // Delete
        repo.delete(&feature.id).await.unwrap();
        let deleted = repo.get(&feature.id).await.unwrap();
        assert!(deleted.is_none());
    }
}
