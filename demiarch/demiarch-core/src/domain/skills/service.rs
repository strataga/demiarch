//! skills domain service

use super::{entity::SkillsEntity, repository::SkillsRepository};
use anyhow::Result;

pub struct SkillsService {
    repository: Box<dyn SkillsRepository>,
}

impl SkillsService {
    pub fn new(repository: Box<dyn SkillsRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_skills(&self, name: String) -> Result<SkillsEntity> {
        let entity = SkillsEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
