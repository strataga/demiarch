//! code_generation repository trait

use super::entity::CodeGenerationEntity;
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait CodeGenerationRepository: Send + Sync {
    async fn create(&self, entity: &CodeGenerationEntity) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<CodeGenerationEntity>>;
    async fn update(&self, entity: &CodeGenerationEntity) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<CodeGenerationEntity>>;
}
