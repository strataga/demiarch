//! cost domain service

use super::{entity::CostEntity, repository::CostRepository};
use anyhow::Result;

pub struct CostService {
    repository: Box<dyn CostRepository>,
}

impl CostService {
    pub fn new(repository: Box<dyn CostRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_cost(&self, name: String) -> Result<CostEntity> {
        let entity = CostEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
