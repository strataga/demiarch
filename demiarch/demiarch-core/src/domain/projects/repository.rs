//! Project repository trait

use super::entity::Project;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    /// Create a new project
    async fn create(&self, project: &Project) -> Result<()>;

    /// Get a project by ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Project>>;

    /// Get a project by name
    async fn get_by_name(&self, name: &str) -> Result<Option<Project>>;

    /// Update a project
    async fn update(&self, project: &Project) -> Result<()>;

    /// Delete a project
    async fn delete(&self, id: Uuid) -> Result<()>;

    /// List all projects
    async fn list_all(&self) -> Result<Vec<Project>>;
}
