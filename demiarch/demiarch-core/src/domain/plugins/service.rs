//! plugins domain service

use super::{entity::PluginsEntity, repository::PluginsRepository};
use anyhow::Result;

pub struct PluginsService {
    repository: Box<dyn PluginsRepository>,
}

impl PluginsService {
    pub fn new(repository: Box<dyn PluginsRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_plugins(&self, name: String) -> Result<PluginsEntity> {
        let entity = PluginsEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
