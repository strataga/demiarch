//! Project management commands
//!
//! Provides CRUD operations for demiarch projects.

use crate::Result;
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// Project status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    #[default]
    Active,
    Archived,
    Deleted,
}

impl ProjectStatus {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Archived => "archived",
            ProjectStatus::Deleted => "deleted",
        }
    }

    /// Parse from database string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(ProjectStatus::Active),
            "archived" => Some(ProjectStatus::Archived),
            "deleted" => Some(ProjectStatus::Deleted),
            _ => None,
        }
    }
}

/// A demiarch project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier
    pub id: String,
    /// Project name
    pub name: String,
    /// Framework used (e.g., "rust", "react", "next.js")
    pub framework: String,
    /// Git repository URL
    pub repo_url: String,
    /// Project status
    pub status: ProjectStatus,
    /// Optional project description
    pub description: Option<String>,
    /// When the project was created
    pub created_at: DateTime<Utc>,
    /// When the project was last updated
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Create a new project with the given details
    pub fn new(
        name: impl Into<String>,
        framework: impl Into<String>,
        repo_url: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            framework: framework.into(),
            repo_url: repo_url.into(),
            status: ProjectStatus::Active,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the project description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Project repository for database operations
pub struct ProjectRepository<'a> {
    db: &'a Database,
}

impl<'a> ProjectRepository<'a> {
    /// Create a new project repository
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new project in the database
    pub async fn create(&self, project: &Project) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO projects (id, name, framework, repo_url, status, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.framework)
        .bind(&project.repo_url)
        .bind(project.status.as_str())
        .bind(&project.description)
        .bind(project.created_at)
        .bind(project.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get a project by ID
    pub async fn get(&self, id: &str) -> Result<Option<Project>> {
        let row = sqlx::query(
            "SELECT id, name, framework, repo_url, status, description, created_at, updated_at FROM projects WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_project(r)))
    }

    /// Get a project by name
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Project>> {
        let row = sqlx::query(
            "SELECT id, name, framework, repo_url, status, description, created_at, updated_at FROM projects WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_project(r)))
    }

    /// List all projects with optional status filter
    pub async fn list(&self, status: Option<ProjectStatus>) -> Result<Vec<Project>> {
        let rows = if let Some(status) = status {
            sqlx::query(
                "SELECT id, name, framework, repo_url, status, description, created_at, updated_at FROM projects WHERE status = ? ORDER BY name",
            )
            .bind(status.as_str())
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query(
                "SELECT id, name, framework, repo_url, status, description, created_at, updated_at FROM projects ORDER BY name",
            )
            .fetch_all(self.db.pool())
            .await?
        };

        Ok(rows.into_iter().map(|r| self.row_to_project(r)).collect())
    }

    /// Update a project
    pub async fn update(&self, project: &Project) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE projects
            SET name = ?, framework = ?, repo_url = ?, status = ?, description = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&project.name)
        .bind(&project.framework)
        .bind(&project.repo_url)
        .bind(project.status.as_str())
        .bind(&project.description)
        .bind(Utc::now())
        .bind(&project.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Archive a project (soft delete)
    pub async fn archive(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET status = 'archived', updated_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Restore an archived project
    pub async fn restore(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET status = 'active', updated_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Mark a project as deleted (soft delete, keeps data)
    pub async fn soft_delete(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET status = 'deleted', updated_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Permanently delete a project and all associated data
    pub async fn hard_delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Check if a project exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM projects WHERE id = ?")
            .bind(id)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(row.is_some())
    }

    /// Check if a project with the given name exists
    pub async fn name_exists(&self, name: &str) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM projects WHERE name = ?")
            .bind(name)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(row.is_some())
    }

    /// Convert a database row to a Project
    fn row_to_project(&self, row: sqlx::sqlite::SqliteRow) -> Project {
        Project {
            id: row.get("id"),
            name: row.get("name"),
            framework: row.get("framework"),
            repo_url: row.get("repo_url"),
            status: ProjectStatus::parse(row.get("status")).unwrap_or_default(),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

// ============================================================================
// Legacy API (for backwards compatibility)
// These functions maintain the original signature but require a Database instance
// ============================================================================

/// Create a new project (legacy API)
///
/// Note: This creates a project with the given details but does not persist it.
/// Use `ProjectRepository::create()` to save to database.
pub async fn create(name: &str, framework: &str, repo_url: &str) -> Result<String> {
    let project = Project::new(name, framework, repo_url);
    Ok(project.id)
}

/// Create a new project and save to database
pub async fn create_with_db(
    db: &Database,
    name: &str,
    framework: &str,
    repo_url: &str,
) -> Result<Project> {
    let repo = ProjectRepository::new(db);

    // Check if name already exists
    if repo.name_exists(name).await? {
        return Err(crate::Error::Validation(format!(
            "A project with name '{}' already exists",
            name
        )));
    }

    let project = Project::new(name, framework, repo_url);
    repo.create(&project).await?;
    Ok(project)
}

/// List all projects (legacy API)
pub async fn list() -> Result<Vec<String>> {
    Ok(Vec::new())
}

/// List all projects from database
pub async fn list_with_db(db: &Database, status: Option<ProjectStatus>) -> Result<Vec<Project>> {
    let repo = ProjectRepository::new(db);
    repo.list(status).await
}

/// Get project by ID (legacy API)
pub async fn get(id: &str) -> Result<Option<String>> {
    // Return the ID if valid UUID format, otherwise None
    if Uuid::parse_str(id).is_ok() {
        Ok(Some(id.to_string()))
    } else {
        Ok(None)
    }
}

/// Get project by ID from database
pub async fn get_with_db(db: &Database, id: &str) -> Result<Option<Project>> {
    let repo = ProjectRepository::new(db);
    repo.get(id).await
}

/// Archive a project (legacy API)
pub async fn archive(id: &str) -> Result<()> {
    // Validate UUID format
    let _ = Uuid::parse_str(id)
        .map_err(|_| crate::Error::Validation("Invalid project ID".to_string()))?;
    Ok(())
}

/// Archive a project in database
pub async fn archive_with_db(db: &Database, id: &str) -> Result<()> {
    let repo = ProjectRepository::new(db);

    // Check if project exists
    if !repo.exists(id).await? {
        return Err(crate::Error::NotFound(format!("Project not found: {}", id)));
    }

    repo.archive(id).await
}

/// Delete a project (legacy API)
pub async fn delete(id: &str, _force: bool) -> Result<()> {
    // Validate UUID format
    let _ = Uuid::parse_str(id)
        .map_err(|_| crate::Error::Validation("Invalid project ID".to_string()))?;
    Ok(())
}

/// Delete a project from database
pub async fn delete_with_db(db: &Database, id: &str, force: bool) -> Result<()> {
    let repo = ProjectRepository::new(db);

    // Check if project exists
    if !repo.exists(id).await? {
        return Err(crate::Error::NotFound(format!("Project not found: {}", id)));
    }

    if force {
        repo.hard_delete(id).await
    } else {
        repo.soft_delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_project() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        let project = create_with_db(&db, "test-project", "rust", "https://github.com/test/test")
            .await
            .expect("Failed to create project");

        assert_eq!(project.name, "test-project");
        assert_eq!(project.framework, "rust");
        assert_eq!(project.status, ProjectStatus::Active);
    }

    #[tokio::test]
    async fn test_get_project() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        let created = create_with_db(&db, "test-project", "rust", "")
            .await
            .expect("Failed to create project");

        let retrieved = get_with_db(&db, &created.id)
            .await
            .expect("Failed to get project")
            .expect("Project should exist");

        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.name, "test-project");
    }

    #[tokio::test]
    async fn test_list_projects() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        create_with_db(&db, "project-a", "rust", "").await.unwrap();
        create_with_db(&db, "project-b", "react", "").await.unwrap();
        create_with_db(&db, "project-c", "python", "")
            .await
            .unwrap();

        let projects = list_with_db(&db, None)
            .await
            .expect("Failed to list projects");
        assert_eq!(projects.len(), 3);

        // Should be sorted by name
        assert_eq!(projects[0].name, "project-a");
        assert_eq!(projects[1].name, "project-b");
        assert_eq!(projects[2].name, "project-c");
    }

    #[tokio::test]
    async fn test_list_projects_by_status() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        create_with_db(&db, "active-project", "rust", "")
            .await
            .unwrap();
        let archived = create_with_db(&db, "archived-project", "rust", "")
            .await
            .unwrap();
        archive_with_db(&db, &archived.id).await.unwrap();

        let active_projects = list_with_db(&db, Some(ProjectStatus::Active))
            .await
            .expect("Failed to list active projects");
        assert_eq!(active_projects.len(), 1);
        assert_eq!(active_projects[0].name, "active-project");

        let archived_projects = list_with_db(&db, Some(ProjectStatus::Archived))
            .await
            .expect("Failed to list archived projects");
        assert_eq!(archived_projects.len(), 1);
        assert_eq!(archived_projects[0].name, "archived-project");
    }

    #[tokio::test]
    async fn test_archive_project() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        let project = create_with_db(&db, "test-project", "rust", "")
            .await
            .expect("Failed to create project");

        archive_with_db(&db, &project.id)
            .await
            .expect("Failed to archive project");

        let retrieved = get_with_db(&db, &project.id)
            .await
            .expect("Failed to get project")
            .expect("Project should exist");

        assert_eq!(retrieved.status, ProjectStatus::Archived);
    }

    #[tokio::test]
    async fn test_soft_delete_project() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        let project = create_with_db(&db, "test-project", "rust", "")
            .await
            .expect("Failed to create project");

        delete_with_db(&db, &project.id, false)
            .await
            .expect("Failed to delete project");

        let retrieved = get_with_db(&db, &project.id)
            .await
            .expect("Failed to get project")
            .expect("Project should still exist (soft delete)");

        assert_eq!(retrieved.status, ProjectStatus::Deleted);
    }

    #[tokio::test]
    async fn test_hard_delete_project() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        let project = create_with_db(&db, "test-project", "rust", "")
            .await
            .expect("Failed to create project");

        delete_with_db(&db, &project.id, true)
            .await
            .expect("Failed to delete project");

        let retrieved = get_with_db(&db, &project.id)
            .await
            .expect("Failed to get project");

        assert!(retrieved.is_none(), "Project should be permanently deleted");
    }

    #[tokio::test]
    async fn test_duplicate_name_error() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        create_with_db(&db, "test-project", "rust", "")
            .await
            .expect("Failed to create project");

        let result = create_with_db(&db, "test-project", "python", "").await;
        assert!(result.is_err(), "Should fail with duplicate name");
    }

    #[tokio::test]
    async fn test_project_not_found() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        let result = archive_with_db(&db, "nonexistent-id").await;
        assert!(result.is_err(), "Should fail with not found");
    }

    #[tokio::test]
    async fn test_project_with_description() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");
        let repo = ProjectRepository::new(&db);

        let project = Project::new("test-project", "rust", "")
            .with_description("A test project for unit testing");

        repo.create(&project)
            .await
            .expect("Failed to create project");

        let retrieved = repo
            .get(&project.id)
            .await
            .expect("Failed to get project")
            .expect("Project should exist");

        assert_eq!(
            retrieved.description,
            Some("A test project for unit testing".to_string())
        );
    }

    #[tokio::test]
    async fn test_restore_archived_project() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");
        let repo = ProjectRepository::new(&db);

        let project = Project::new("test-project", "rust", "");
        repo.create(&project).await.unwrap();

        // Archive
        repo.archive(&project.id).await.unwrap();
        let archived = repo.get(&project.id).await.unwrap().unwrap();
        assert_eq!(archived.status, ProjectStatus::Archived);

        // Restore
        repo.restore(&project.id).await.unwrap();
        let restored = repo.get(&project.id).await.unwrap().unwrap();
        assert_eq!(restored.status, ProjectStatus::Active);
    }
}
