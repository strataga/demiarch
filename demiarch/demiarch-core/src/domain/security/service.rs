//! security domain service

use super::{entity::SecurityEntity, repository::SecurityRepository};
use anyhow::Result;

pub struct SecurityService {
    repository: Box<dyn SecurityRepository>,
}

impl SecurityService {
    pub fn new(repository: Box<dyn SecurityRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_security(&self, name: String) -> Result<SecurityEntity> {
        let entity = SecurityEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
