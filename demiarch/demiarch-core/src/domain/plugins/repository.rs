//! plugins repository trait

use super::entity::PluginsEntity;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait PluginsRepository: Send + Sync {
    async fn create(&self, entity: &PluginsEntity) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PluginsEntity>>;
    async fn update(&self, entity: &PluginsEntity) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<PluginsEntity>>;
}
