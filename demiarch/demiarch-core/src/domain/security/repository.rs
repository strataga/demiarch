//! security repository trait

use super::entity::SecurityEntity;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait SecurityRepository: Send + Sync {
    async fn create(&self, entity: &SecurityEntity) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<SecurityEntity>>;
    async fn update(&self, entity: &SecurityEntity) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<SecurityEntity>>;
}
