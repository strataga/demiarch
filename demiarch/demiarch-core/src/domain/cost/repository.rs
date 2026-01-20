//! cost repository trait

use super::entity::CostEntity;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait CostRepository: Send + Sync {
    async fn create(&self, entity: &CostEntity) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<CostEntity>>;
    async fn update(&self, entity: &CostEntity) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<CostEntity>>;
}
