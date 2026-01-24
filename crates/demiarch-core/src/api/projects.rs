//! Projects API
//!
//! High-level async functions for project operations.

use crate::commands::project::{Project, ProjectRepository, ProjectStatus};
use crate::Result;
use serde::{Deserialize, Serialize};

use super::get_database;

/// Project summary DTO for the GUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub status: String,
    pub description: Option<String>,
    pub path: Option<String>,
    pub feature_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Project> for ProjectSummary {
    fn from(p: Project) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            framework: p.framework.clone(),
            status: p.status.as_str().to_string(),
            description: p.description.clone(),
            path: p.path.clone(),
            feature_count: 0, // Will be populated separately if needed
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// Create project request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub framework: String,
    pub repo_url: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
}

/// List all projects
pub async fn list(status: Option<&str>) -> Result<Vec<ProjectSummary>> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);

    let status_filter = status.and_then(ProjectStatus::parse);
    let projects = repo.list(status_filter).await?;

    Ok(projects.into_iter().map(ProjectSummary::from).collect())
}

/// Get a project by ID
pub async fn get(id: &str) -> Result<Option<ProjectSummary>> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);

    let project = repo.get(id).await?;
    Ok(project.map(ProjectSummary::from))
}

/// Get a project by path
pub async fn get_by_path(path: &str) -> Result<Option<ProjectSummary>> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);

    let project = repo.get_by_path(path).await?;
    Ok(project.map(ProjectSummary::from))
}

/// Create a new project
pub async fn create(request: CreateProjectRequest) -> Result<ProjectSummary> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);

    // Check for duplicate name
    if repo.name_exists(&request.name).await? {
        return Err(crate::Error::Validation(format!(
            "Project with name '{}' already exists",
            request.name
        )));
    }

    let repo_url = request.repo_url.unwrap_or_default();
    let mut project = Project::new(&request.name, &request.framework, &repo_url);

    if let Some(desc) = request.description {
        project = project.with_description(&desc);
    }

    if let Some(path) = request.path {
        project = project.with_path(&path);
    }

    repo.create(&project).await?;

    Ok(ProjectSummary::from(project))
}

/// Archive a project (soft delete)
pub async fn archive(id: &str) -> Result<()> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);
    repo.archive(id).await
}

/// Restore an archived project
pub async fn restore(id: &str) -> Result<()> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);
    repo.restore(id).await
}

/// Delete a project
pub async fn delete(id: &str, hard: bool) -> Result<()> {
    let db = get_database().await?;
    let repo = ProjectRepository::new(&db);

    if hard {
        repo.hard_delete(id).await
    } else {
        repo.soft_delete(id).await
    }
}
