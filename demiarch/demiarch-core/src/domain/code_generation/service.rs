//! code_generation domain service

use super::{entity::CodeGenerationEntity, repository::CodeGenerationRepository};
use anyhow::Result;
use chrono::Utc;

pub struct CodeGenerationService {
    repository: Box<dyn CodeGenerationRepository>,
}

impl CodeGenerationService {
    pub fn new(repository: Box<dyn CodeGenerationRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_code_generation(&self, name: String) -> Result<CodeGenerationEntity> {
        let now = Utc::now();
        let entity = CodeGenerationEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: now,
            updated_at: now,
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}
