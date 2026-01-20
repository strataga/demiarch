//! context domain service

use super::{entity::ContextEntity, repository::ContextRepository};
use anyhow::Result;

pub struct ContextService {
    repository: Box<dyn ContextRepository>,
}

impl ContextService {
    pub fn new(repository: Box<dyn ContextRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_context(&self, name: String) -> Result<ContextEntity> {
        let entity = ContextEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
