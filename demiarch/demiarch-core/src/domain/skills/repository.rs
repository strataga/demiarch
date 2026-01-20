//! skills repository trait

use super::entity::SkillsEntity;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait SkillsRepository: Send + Sync {
    async fn create(&self, entity: &SkillsEntity) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<SkillsEntity>>;
    async fn update(&self, entity: &SkillsEntity) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<SkillsEntity>>;
}
