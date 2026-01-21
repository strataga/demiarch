//! Project domain service

use super::{entity::Project, repository::ProjectRepository, value_object::ProjectName};
use anyhow::{anyhow, Error, Result};
use std::collections::HashSet;

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
        ctx: &SecurityContext,
        name: String,
        description: Option<String>,
    ) -> Result<Project> {
        ctx.require(ProjectScope::WriteProjects)?;

        let validated_name = ProjectName::new(name).map_err(Error::msg)?;
        let project = Project::new(validated_name.as_str().to_string(), description);
        self.repository.create(&project).await?;
        Ok(project)
    }

    /// Get project by ID
    pub async fn get_project(
        &self,
        ctx: &SecurityContext,
        id: uuid::Uuid,
    ) -> Result<Option<Project>> {
        ctx.require(ProjectScope::ReadProjects)?;

        self.repository.get_by_id(id).await
    }

    /// List all projects
    pub async fn list_projects(&self, ctx: &SecurityContext) -> Result<Vec<Project>> {
        ctx.require(ProjectScope::ReadProjects)?;

        self.repository.list_all().await
    }
}

/// Minimal security context to enforce authorization on domain operations
#[derive(Debug, Clone)]
pub struct SecurityContext {
    user_id: Option<uuid::Uuid>,
    scopes: HashSet<ProjectScope>,
}

impl SecurityContext {
    pub fn new<I: IntoIterator<Item = ProjectScope>>(
        user_id: Option<uuid::Uuid>,
        scopes: I,
    ) -> Self {
        let scopes_set = scopes.into_iter().collect();
        Self {
            user_id,
            scopes: scopes_set,
        }
    }

    pub fn require(&self, scope: ProjectScope) -> Result<()> {
        if !self.scopes.contains(&scope) {
            return Err(anyhow!(
                "Missing required project scope: {:?} for user {:?}",
                scope,
                self.user_id
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectScope {
    ReadProjects,
    WriteProjects,
    ManageProjects,
}
