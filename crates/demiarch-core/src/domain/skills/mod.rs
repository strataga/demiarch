//! Skills domain module
//!
//! Provides skill extraction and validation from debugging sessions.
//! Skills are learned patterns that can be reused in future tasks.

pub mod entity;
pub mod extraction;
pub mod validation;

use crate::domain::knowledge::KnowledgeEvent;
use crate::domain::locking::guard::LockGuard;
use entity::SkillsEntity;
use extraction::DebugSkillExtractor;
use std::sync::Arc;
use validation::SkillValidator;

// Re-export main types
pub use entity::{SkillContext, SkillSourceInfo, SkillType, SkillsEntity as Skill};
pub use extraction::DebugSkillExtractor as Extractor;
pub use validation::SkillValidator as Validator;

/// Skill extraction service
///
/// Processes session logs to extract reusable skills from debugging patterns.
pub struct SkillExtractor {
    /// Minimum number of successful uses before considering extraction
    min_observation_count: u32,
    /// Minimum author reputation score before trusting their skills
    min_author_reputation: f32,
    /// The underlying debug extractor
    debug_extractor: DebugSkillExtractor,
    /// The validator for extracted skills
    validator: SkillValidator,
}

impl Default for SkillExtractor {
    fn default() -> Self {
        Self::new(2, 0.5)
    }
}

impl SkillExtractor {
    /// Create a new skill extractor
    pub fn new(min_observation_count: u32, min_author_reputation: f32) -> Self {
        Self {
            min_observation_count,
            min_author_reputation,
            debug_extractor: DebugSkillExtractor::new()
                .with_min_occurrences(min_observation_count)
                .with_min_confidence(min_author_reputation),
            validator: SkillValidator::new(0.6, 2),
        }
    }

    /// Configure minimum observations
    pub fn with_min_observations(mut self, count: u32) -> Self {
        self.min_observation_count = count;
        self.debug_extractor = self.debug_extractor.with_min_occurrences(count);
        self
    }

    /// Configure minimum reputation
    pub fn with_min_reputation(mut self, reputation: f32) -> Self {
        self.min_author_reputation = reputation;
        self.debug_extractor = self.debug_extractor.with_min_confidence(reputation);
        self
    }

    /// Configure the validator
    pub fn with_validator(mut self, validator: SkillValidator) -> Self {
        self.validator = validator;
        self
    }

    /// Process a debugging session to extract potential skills
    ///
    /// Analyzes the session log (a sequence of knowledge events) to identify
    /// patterns that can be extracted as reusable skills.
    ///
    /// # Arguments
    /// * `session_log` - The knowledge events from the session
    /// * `lock` - A lock guard ensuring exclusive access during extraction
    ///
    /// # Returns
    /// A list of extracted skills, or an error if extraction fails
    pub async fn extract_from_session(
        &self,
        session_log: Arc<Vec<KnowledgeEvent>>,
        lock: LockGuard,
    ) -> Result<Vec<SkillsEntity>, SkillExtractionError> {
        // Validate the lock
        if !lock.is_valid() {
            return Err(SkillExtractionError::LockExpired);
        }

        // Check minimum event count
        if session_log.len() < 2 {
            return Ok(vec![]); // Not enough events to extract patterns
        }

        // Extract raw skills using the debug extractor
        let raw_skills = self.debug_extractor.extract(session_log);

        // Filter and validate extracted skills
        let validated_skills: Vec<SkillsEntity> = raw_skills
            .into_iter()
            .filter(|skill| {
                // Check observation count
                if skill.observation_count < self.min_observation_count {
                    return false;
                }

                // Validate the skill
                match self.validator.validate(skill) {
                    Ok(()) => true,
                    Err(reason) => {
                        tracing::debug!(
                            skill_name = %skill.name,
                            reason = %reason,
                            "Skill failed validation"
                        );
                        false
                    }
                }
            })
            .collect();

        Ok(validated_skills)
    }

    /// Extract skills without requiring a lock (for testing or batch processing)
    pub fn extract_unlocked(&self, events: &[KnowledgeEvent]) -> Vec<SkillsEntity> {
        self.debug_extractor.extract(Arc::new(events.to_vec()))
    }

    /// Identify effective error-solution pairs from events
    pub fn identify_solutions(&self, events: &[KnowledgeEvent]) -> Vec<(String, String)> {
        self.debug_extractor.identify_effective_solutions(events)
    }
}

/// Errors that can occur during skill extraction
#[derive(thiserror::Error, Debug)]
pub enum SkillExtractionError {
    /// The lock expired before skill extraction completed
    #[error("Lock expired before skill extraction completed")]
    LockExpired,

    /// Validation failed for extracted skills
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Not enough observations to extract a skill
    #[error("Insufficient observations: {0}")]
    InsufficientObservations(String),

    /// No extractable patterns found
    #[error("No patterns found in session")]
    NoPatterns,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locking::types::{LockInfo, ResourceType};
    use chrono::Utc;
    use std::time::Duration;

    fn create_mock_lock() -> LockGuard {
        // Create a mock lock that is always valid
        let info = LockInfo::new(
            ResourceType::Session,
            "test-session".to_string(),
            "test-holder".to_string(),
            Some(Duration::from_secs(3600)),
        );
        LockGuard::new_test(info)
    }

    fn create_test_events() -> Vec<KnowledgeEvent> {
        use crate::domain::knowledge::{EntityType, RelationshipType};
        let now = Utc::now();
        vec![
            KnowledgeEvent::EntityUpdated {
                entity_id: "entity-1".to_string(),
                changes: vec!["Error: compilation failed".to_string()],
                timestamp: now,
            },
            KnowledgeEvent::EntityCreated {
                entity_id: "entity-2".to_string(),
                entity_type: EntityType::Library,
                name: "fix-library".to_string(),
                source_skill_id: None,
                timestamp: now + chrono::Duration::seconds(30),
            },
            KnowledgeEvent::RelationshipCreated {
                relationship_id: "rel-1".to_string(),
                source_entity_id: "entity-1".to_string(),
                target_entity_id: "entity-2".to_string(),
                relationship_type: RelationshipType::Uses,
                timestamp: now + chrono::Duration::seconds(60),
            },
            // Add more events to meet minimum thresholds
            KnowledgeEvent::RelationshipCreated {
                relationship_id: "rel-2".to_string(),
                source_entity_id: "entity-2".to_string(),
                target_entity_id: "entity-1".to_string(),
                relationship_type: RelationshipType::Uses,
                timestamp: now + chrono::Duration::seconds(90),
            },
        ]
    }

    #[test]
    fn test_extractor_creation() {
        let extractor = SkillExtractor::new(3, 0.7);
        assert_eq!(extractor.min_observation_count, 3);
        assert_eq!(extractor.min_author_reputation, 0.7);
    }

    #[test]
    fn test_extractor_default() {
        let extractor = SkillExtractor::default();
        assert_eq!(extractor.min_observation_count, 2);
        assert_eq!(extractor.min_author_reputation, 0.5);
    }

    #[test]
    fn test_extract_unlocked() {
        let extractor = SkillExtractor::default().with_min_observations(1);
        let events = create_test_events();
        let skills = extractor.extract_unlocked(&events);

        // May or may not find skills depending on patterns
        let _ = skills;
    }

    #[test]
    fn test_identify_solutions() {
        let extractor = SkillExtractor::default();
        let events = create_test_events();
        let solutions = extractor.identify_solutions(&events);

        // Verify it doesn't panic
        let _ = solutions;
    }

    #[tokio::test]
    async fn test_extract_from_session_empty() {
        let extractor = SkillExtractor::default();
        let lock = create_mock_lock();
        let events = Arc::new(vec![]);

        let result = extractor.extract_from_session(events, lock).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_extract_from_session() {
        let extractor = SkillExtractor::default().with_min_observations(1);
        let lock = create_mock_lock();
        let events = Arc::new(create_test_events());

        let result = extractor.extract_from_session(events, lock).await;
        assert!(result.is_ok());
    }
}
