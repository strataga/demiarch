//! Project domain service

use super::{entity::Project, repository::ProjectRepository};
use anyhow::Result;

pub struct ProjectService {
    repository: Box<dyn ProjectRepository>,
}

impl ProjectService {
    pub fn new(repository: Box<dyn ProjectRepository>) -> Self {
        Self { repository }
    }

    /// Create a new project
    pub async fn create_project(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Project> {
        let project = Project::new(name, description);
        self.repository.create(&project).await?;
        Ok(project)
    }

    /// Get project by ID
    pub async fn get_project(&self, id: uuid::Uuid) -> Result<Option<Project>> {
        self.repository.get_by_id(id).await
    }

    /// List all projects
    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        self.repository.list_all().await
    }
}
