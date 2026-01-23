pub mod entity;
pub mod extraction;
pub mod validation;

use crate::domain::knowledge::event::KnowledgeEvent;
use crate::domain::locking::guard::LockGuard;
use entity::SkillsEntity;
use std::sync::Arc;

/// Skill extraction service
pub struct SkillExtractor {
    /// Minimum number of successful uses before considering extraction
    min_observation_count: u32,

    /// Minimum author reputation score before trusting their skills
    min_author_reputation: f32,
}

impl SkillExtractor {
    pub fn new(min_observation_count: u32, min_author_reputation: f32) -> Self {
        Self {
            min_observation_count,
            min_author_reputation,
        }
    }

    /// Process a debugging session to extract potential skills
    pub async fn extract_from_session(
        &self,
        session_log: Arc<Vec<KnowledgeEvent>>,
        lock: LockGuard,
    ) -> Result<Vec<SkillsEntity>, SkillExtractionError> {
        if !lock.is_valid() {
            return Err(SkillExtractionError::LockExpired);
        }

        // TODO: Implement extraction pipeline
        Ok(vec![])
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SkillExtractionError {
    #[error("Lock expired before skill extraction completed")]
    LockExpired,
    #[error("Validation failed: {0}")]
    Validation(String),
    #[error("Insufficient observations: {0}")]
    InsufficientObservations(String),
}
