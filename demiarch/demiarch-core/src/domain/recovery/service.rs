//! recovery domain service

use super::{entity::RecoveryEntity, repository::RecoveryRepository};
use anyhow::Result;

pub struct RecoveryService {
    repository: Box<dyn RecoveryRepository>,
}

impl RecoveryService {
    pub fn new(repository: Box<dyn RecoveryRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_recovery(&self, name: String) -> Result<RecoveryEntity> {
        let entity = RecoveryEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
