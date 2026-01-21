//! Phase management commands
//!
//! Provides CRUD operations for project phases and phase planning.

use crate::Result;
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// Phase status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PhaseStatus {
    #[default]
    Pending,
    InProgress,
    Complete,
    Skipped,
}

impl PhaseStatus {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            PhaseStatus::Pending => "pending",
            PhaseStatus::InProgress => "in_progress",
            PhaseStatus::Complete => "complete",
            PhaseStatus::Skipped => "skipped",
        }
    }

    /// Parse from database string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(PhaseStatus::Pending),
            "in_progress" => Some(PhaseStatus::InProgress),
            "complete" => Some(PhaseStatus::Complete),
            "skipped" => Some(PhaseStatus::Skipped),
            _ => None,
        }
    }
}

/// A project phase for organizing features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    /// Unique phase identifier
    pub id: String,
    /// Associated project ID
    pub project_id: String,
    /// Phase name
    pub name: String,
    /// Phase description
    pub description: Option<String>,
    /// Phase status
    pub status: PhaseStatus,
    /// Order index for display ordering
    pub order_index: i32,
    /// When the phase was created
    pub created_at: DateTime<Utc>,
    /// When the phase was last updated
    pub updated_at: DateTime<Utc>,
}

impl Phase {
    /// Create a new phase
    pub fn new(project_id: impl Into<String>, name: impl Into<String>, order_index: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            name: name.into(),
            description: None,
            status: PhaseStatus::Pending,
            order_index,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: PhaseStatus) -> Self {
        self.status = status;
        self
    }
}

/// Default phases for a project
pub const DEFAULT_PHASES: &[(&str, &str, i32)] = &[
    ("Discovery", "Requirements gathering and ideation", 0),
    ("Planning", "Technical design and architecture", 1),
    ("Building", "Implementation and development", 2),
    ("Complete", "Finished and deployed", 3),
];

/// Phase repository for database operations
pub struct PhaseRepository<'a> {
    db: &'a Database,
}

impl<'a> PhaseRepository<'a> {
    /// Create a new phase repository
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new phase in the database
    pub async fn create(&self, phase: &Phase) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO phases (id, project_id, name, description, status, order_index, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&phase.id)
        .bind(&phase.project_id)
        .bind(&phase.name)
        .bind(&phase.description)
        .bind(phase.status.as_str())
        .bind(phase.order_index)
        .bind(phase.created_at)
        .bind(phase.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get a phase by ID
    pub async fn get(&self, id: &str) -> Result<Option<Phase>> {
        let row = sqlx::query(
            "SELECT id, project_id, name, description, status, order_index, created_at, updated_at FROM phases WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_phase(r)))
    }

    /// Get a phase by name for a project
    pub async fn get_by_name(&self, project_id: &str, name: &str) -> Result<Option<Phase>> {
        let row = sqlx::query(
            "SELECT id, project_id, name, description, status, order_index, created_at, updated_at FROM phases WHERE project_id = ? AND name = ?",
        )
        .bind(project_id)
        .bind(name)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_phase(r)))
    }

    /// List phases for a project (ordered by order_index)
    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Phase>> {
        let rows = sqlx::query(
            "SELECT id, project_id, name, description, status, order_index, created_at, updated_at FROM phases WHERE project_id = ? ORDER BY order_index ASC",
        )
        .bind(project_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|r| self.row_to_phase(r)).collect())
    }

    /// Update a phase
    pub async fn update(&self, phase: &Phase) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE phases
            SET name = ?, description = ?, status = ?, order_index = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&phase.name)
        .bind(&phase.description)
        .bind(phase.status.as_str())
        .bind(phase.order_index)
        .bind(Utc::now())
        .bind(&phase.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Update phase status
    pub async fn update_status(&self, id: &str, status: PhaseStatus) -> Result<()> {
        sqlx::query("UPDATE phases SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Delete a phase
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM phases WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Count features in a phase
    pub async fn count_features(&self, phase_id: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM features WHERE phase_id = ?")
            .bind(phase_id)
            .fetch_one(self.db.pool())
            .await?;

        Ok(row.0)
    }

    /// Check if a phase exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM phases WHERE id = ?")
            .bind(id)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(row.is_some())
    }

    /// Convert a database row to a Phase
    fn row_to_phase(&self, row: sqlx::sqlite::SqliteRow) -> Phase {
        Phase {
            id: row.get("id"),
            project_id: row.get("project_id"),
            name: row.get("name"),
            description: row.get("description"),
            status: PhaseStatus::parse(row.get("status")).unwrap_or_default(),
            order_index: row.get("order_index"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

// ============================================================================
// Public API functions
// ============================================================================

/// Create default phases for a project
pub async fn create_default_phases(db: &Database, project_id: &str) -> Result<Vec<Phase>> {
    let repo = PhaseRepository::new(db);
    let mut phases = Vec::new();

    for (name, description, order_index) in DEFAULT_PHASES {
        let phase = Phase::new(project_id, *name, *order_index).with_description(*description);

        repo.create(&phase).await?;
        phases.push(phase);
    }

    Ok(phases)
}

/// Create a custom phase for a project
pub async fn create_phase(
    db: &Database,
    project_id: &str,
    name: &str,
    description: Option<&str>,
    order_index: i32,
) -> Result<Phase> {
    let repo = PhaseRepository::new(db);

    let mut phase = Phase::new(project_id, name, order_index);
    if let Some(desc) = description {
        phase = phase.with_description(desc);
    }

    repo.create(&phase).await?;
    Ok(phase)
}

/// List all phases for a project
pub async fn list_phases(db: &Database, project_id: &str) -> Result<Vec<Phase>> {
    let repo = PhaseRepository::new(db);
    repo.list_by_project(project_id).await
}

/// Get a phase by ID
pub async fn get_phase(db: &Database, id: &str) -> Result<Option<Phase>> {
    let repo = PhaseRepository::new(db);
    repo.get(id).await
}

/// Update phase status
pub async fn update_phase_status(db: &Database, id: &str, status: PhaseStatus) -> Result<()> {
    let repo = PhaseRepository::new(db);

    if !repo.exists(id).await? {
        return Err(crate::Error::NotFound(format!("Phase not found: {}", id)));
    }

    repo.update_status(id, status).await
}

/// Delete a phase
pub async fn delete_phase(db: &Database, id: &str) -> Result<()> {
    let repo = PhaseRepository::new(db);

    if !repo.exists(id).await? {
        return Err(crate::Error::NotFound(format!("Phase not found: {}", id)));
    }

    repo.delete(id).await
}

/// Get phases with feature counts
pub async fn get_phases_with_counts(
    db: &Database,
    project_id: &str,
) -> Result<Vec<PhaseWithCount>> {
    let repo = PhaseRepository::new(db);
    let phases = repo.list_by_project(project_id).await?;

    let mut result = Vec::new();
    for phase in phases {
        let count = repo.count_features(&phase.id).await?;
        result.push(PhaseWithCount {
            phase,
            feature_count: count,
        });
    }

    Ok(result)
}

/// Phase with feature count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseWithCount {
    /// The phase
    #[serde(flatten)]
    pub phase: Phase,
    /// Number of features in this phase
    pub feature_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::project::{Project, ProjectRepository};

    #[test]
    fn test_phase_status_parse() {
        assert_eq!(PhaseStatus::parse("pending"), Some(PhaseStatus::Pending));
        assert_eq!(
            PhaseStatus::parse("in_progress"),
            Some(PhaseStatus::InProgress)
        );
        assert_eq!(PhaseStatus::parse("complete"), Some(PhaseStatus::Complete));
        assert_eq!(PhaseStatus::parse("skipped"), Some(PhaseStatus::Skipped));
        assert_eq!(PhaseStatus::parse("invalid"), None);
    }

    #[test]
    fn test_phase_new() {
        let phase = Phase::new("proj-123", "Test Phase", 0);

        assert!(!phase.id.is_empty());
        assert_eq!(phase.project_id, "proj-123");
        assert_eq!(phase.name, "Test Phase");
        assert_eq!(phase.status, PhaseStatus::Pending);
        assert_eq!(phase.order_index, 0);
    }

    #[test]
    fn test_phase_builders() {
        let phase = Phase::new("proj-123", "Test Phase", 1)
            .with_description("A test phase")
            .with_status(PhaseStatus::InProgress);

        assert_eq!(phase.description, Some("A test phase".to_string()));
        assert_eq!(phase.status, PhaseStatus::InProgress);
    }

    #[tokio::test]
    async fn test_phase_repository_crud() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        // Create a project first
        let project = Project::new("test-project", "rust", "");
        let project_repo = ProjectRepository::new(&db);
        project_repo.create(&project).await.unwrap();

        let repo = PhaseRepository::new(&db);

        // Create
        let phase = Phase::new(&project.id, "Test Phase", 0).with_description("A test phase");
        repo.create(&phase).await.unwrap();

        // Read
        let retrieved = repo.get(&phase.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Phase");

        // List
        let phases = repo.list_by_project(&project.id).await.unwrap();
        assert_eq!(phases.len(), 1);

        // Update status
        repo.update_status(&phase.id, PhaseStatus::InProgress)
            .await
            .unwrap();
        let updated = repo.get(&phase.id).await.unwrap().unwrap();
        assert_eq!(updated.status, PhaseStatus::InProgress);

        // Delete
        repo.delete(&phase.id).await.unwrap();
        let deleted = repo.get(&phase.id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_create_default_phases() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        // Create a project first
        let project = Project::new("test-project", "rust", "");
        let project_repo = ProjectRepository::new(&db);
        project_repo.create(&project).await.unwrap();

        // Create default phases
        let phases = create_default_phases(&db, &project.id).await.unwrap();

        assert_eq!(phases.len(), 4);
        assert_eq!(phases[0].name, "Discovery");
        assert_eq!(phases[1].name, "Planning");
        assert_eq!(phases[2].name, "Building");
        assert_eq!(phases[3].name, "Complete");
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        // Create a project and phases
        let project = Project::new("test-project", "rust", "");
        let project_repo = ProjectRepository::new(&db);
        project_repo.create(&project).await.unwrap();

        create_default_phases(&db, &project.id).await.unwrap();

        // Get by name
        let repo = PhaseRepository::new(&db);
        let phase = repo.get_by_name(&project.id, "Building").await.unwrap();

        assert!(phase.is_some());
        assert_eq!(phase.unwrap().order_index, 2);
    }
}
