//! Skill entity for domain-driven skill extraction
//!
//! This entity represents a learned skill extracted from debugging sessions
//! and knowledge events, distinct from the LLM-based skill extraction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A skill extracted from debugging sessions via pattern analysis
///
/// SkillsEntity represents patterns discovered through analyzing
/// KnowledgeEvents - tracking error-solution pairs, successful approaches,
/// and repeatable techniques learned from session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsEntity {
    /// Unique identifier
    pub id: String,
    /// Human-readable name for the skill
    pub name: String,
    /// Detailed description of the skill
    pub description: String,
    /// The type of skill
    pub skill_type: SkillType,
    /// Pattern template that can be reused
    pub pattern: String,
    /// Context in which this skill applies
    pub context: SkillContext,
    /// Success rate when applied (0.0 - 1.0)
    pub success_rate: f32,
    /// Number of times this skill has been observed
    pub observation_count: u32,
    /// Number of unique contexts this skill has been tested in
    pub tested_contexts: u32,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// When the skill was first observed
    pub created_at: DateTime<Utc>,
    /// When the skill was last updated
    pub updated_at: DateTime<Utc>,
    /// Source information
    pub source: SkillSourceInfo,
}

impl SkillsEntity {
    /// Create a new skill entity
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        skill_type: SkillType,
        pattern: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            skill_type,
            pattern: pattern.into(),
            context: SkillContext::default(),
            success_rate: 0.5, // Start neutral
            observation_count: 1,
            tested_contexts: 1,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            source: SkillSourceInfo::default(),
        }
    }

    /// Create a skill from an error-solution pair
    pub fn from_error_solution(
        error_pattern: impl Into<String>,
        solution: impl Into<String>,
        context_description: impl Into<String>,
    ) -> Self {
        let error = error_pattern.into();
        let sol = solution.into();

        let name = format!("Fix: {}", truncate(&error, 50));
        let description = format!(
            "When encountering '{}', apply: {}",
            truncate(&error, 100),
            truncate(&sol, 200)
        );

        Self::new(name, description, SkillType::ErrorSolution, sol).with_context(SkillContext {
            error_pattern: Some(error),
            description: Some(context_description.into()),
            ..Default::default()
        })
    }

    /// Create a skill from a repeated technique
    pub fn from_technique(
        technique_name: impl Into<String>,
        steps: Vec<String>,
        applicability: impl Into<String>,
    ) -> Self {
        let name = technique_name.into();
        let pattern = steps.join("\n");
        let description = format!("Technique applicable when: {}", applicability.into());

        Self::new(&name, description, SkillType::Technique, pattern)
    }

    /// Set the skill context
    pub fn with_context(mut self, context: SkillContext) -> Self {
        self.context = context;
        self
    }

    /// Add tags to the skill
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set source information
    pub fn with_source(mut self, source: SkillSourceInfo) -> Self {
        self.source = source;
        self
    }

    /// Record a successful application of this skill
    pub fn record_success(&mut self) {
        self.observation_count += 1;
        // Update success rate with exponential moving average
        let weight = 0.1;
        self.success_rate = self.success_rate * (1.0 - weight) + weight;
        self.updated_at = Utc::now();
    }

    /// Record a failed application of this skill
    pub fn record_failure(&mut self) {
        self.observation_count += 1;
        // Update success rate with exponential moving average
        let weight = 0.1;
        self.success_rate *= 1.0 - weight;
        self.updated_at = Utc::now();
    }

    /// Record application in a new context
    pub fn record_new_context(&mut self, success: bool) {
        self.tested_contexts += 1;
        if success {
            self.record_success();
        } else {
            self.record_failure();
        }
    }

    /// Check if this skill is considered reliable
    pub fn is_reliable(&self) -> bool {
        self.success_rate >= 0.7 && self.observation_count >= 3
    }

    /// Check if this skill needs more validation
    pub fn needs_validation(&self) -> bool {
        self.observation_count < 3 || self.tested_contexts < 2
    }

    /// Merge with another observation of the same skill
    pub fn merge_observation(&mut self, other: &SkillsEntity) {
        // Combine observation counts
        let total = self.observation_count + other.observation_count;

        // Weighted average of success rates
        self.success_rate = (self.success_rate * self.observation_count as f32
            + other.success_rate * other.observation_count as f32)
            / total as f32;

        self.observation_count = total;
        self.tested_contexts = self.tested_contexts.max(other.tested_contexts);

        // Merge tags
        for tag in &other.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }

        self.updated_at = Utc::now();
    }

    /// Check if this skill matches a search query
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        self.name.to_lowercase().contains(&query_lower)
            || self.description.to_lowercase().contains(&query_lower)
            || self.pattern.to_lowercase().contains(&query_lower)
            || self
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&query_lower))
            || self
                .context
                .error_pattern
                .as_ref()
                .map(|e| e.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
    }
}

/// Types of skills that can be extracted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillType {
    /// Error pattern matched to a solution
    ErrorSolution,
    /// Repeated debugging technique
    Technique,
    /// Architectural pattern
    ArchitecturePattern,
    /// Code transformation pattern
    CodeTransform,
    /// Configuration pattern
    ConfigPattern,
    /// Testing strategy
    TestingStrategy,
}

impl SkillType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ErrorSolution => "error_solution",
            Self::Technique => "technique",
            Self::ArchitecturePattern => "architecture_pattern",
            Self::CodeTransform => "code_transform",
            Self::ConfigPattern => "config_pattern",
            Self::TestingStrategy => "testing_strategy",
        }
    }
}

/// Context information for when a skill applies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillContext {
    /// Error pattern this skill addresses (if applicable)
    pub error_pattern: Option<String>,
    /// Description of when this skill applies
    pub description: Option<String>,
    /// Programming language (if specific)
    pub language: Option<String>,
    /// Framework (if specific)
    pub framework: Option<String>,
    /// Preconditions for applying this skill
    pub preconditions: Vec<String>,
    /// Postconditions after applying this skill
    pub postconditions: Vec<String>,
}

/// Source information for how a skill was discovered
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillSourceInfo {
    /// Session IDs where this skill was observed
    pub session_ids: Vec<String>,
    /// Project IDs where this skill was applied
    pub project_ids: Vec<String>,
    /// Original event IDs that led to extraction
    pub event_ids: Vec<String>,
}

impl SkillSourceInfo {
    /// Create new source info from a session
    pub fn from_session(session_id: impl Into<String>) -> Self {
        Self {
            session_ids: vec![session_id.into()],
            ..Default::default()
        }
    }

    /// Add a session ID
    pub fn add_session(&mut self, session_id: impl Into<String>) {
        let id = session_id.into();
        if !self.session_ids.contains(&id) {
            self.session_ids.push(id);
        }
    }

    /// Add a project ID
    pub fn add_project(&mut self, project_id: impl Into<String>) {
        let id = project_id.into();
        if !self.project_ids.contains(&id) {
            self.project_ids.push(id);
        }
    }

    /// Add an event ID
    pub fn add_event(&mut self, event_id: impl Into<String>) {
        self.event_ids.push(event_id.into());
    }
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = SkillsEntity::new(
            "Test Skill",
            "A test skill description",
            SkillType::Technique,
            "Step 1\nStep 2",
        );

        assert!(!skill.id.is_empty());
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.skill_type, SkillType::Technique);
        assert_eq!(skill.observation_count, 1);
    }

    #[test]
    fn test_error_solution_skill() {
        let skill = SkillsEntity::from_error_solution(
            "cannot find module 'foo'",
            "npm install foo",
            "When module is missing",
        );

        assert_eq!(skill.skill_type, SkillType::ErrorSolution);
        assert!(skill.context.error_pattern.is_some());
    }

    #[test]
    fn test_success_tracking() {
        let mut skill = SkillsEntity::new(
            "Tracked Skill",
            "Description",
            SkillType::Technique,
            "Pattern",
        );

        let initial_rate = skill.success_rate;
        skill.record_success();
        assert!(skill.success_rate > initial_rate);
        assert_eq!(skill.observation_count, 2);
    }

    #[test]
    fn test_failure_tracking() {
        let mut skill = SkillsEntity::new(
            "Tracked Skill",
            "Description",
            SkillType::Technique,
            "Pattern",
        );

        let initial_rate = skill.success_rate;
        skill.record_failure();
        assert!(skill.success_rate < initial_rate);
    }

    #[test]
    fn test_merge_observations() {
        let mut skill1 = SkillsEntity::new("Skill", "Description", SkillType::Technique, "Pattern")
            .with_tags(vec!["tag1".into()]);
        skill1.success_rate = 0.8;
        skill1.observation_count = 5;

        let mut skill2 = SkillsEntity::new("Skill", "Description", SkillType::Technique, "Pattern")
            .with_tags(vec!["tag2".into()]);
        skill2.success_rate = 0.6;
        skill2.observation_count = 3;

        skill1.merge_observation(&skill2);

        assert_eq!(skill1.observation_count, 8);
        assert!(skill1.success_rate > 0.6 && skill1.success_rate < 0.8);
        assert!(skill1.tags.contains(&"tag1".into()));
        assert!(skill1.tags.contains(&"tag2".into()));
    }

    #[test]
    fn test_reliability() {
        let mut skill = SkillsEntity::new("Skill", "Description", SkillType::Technique, "Pattern");

        skill.success_rate = 0.8;
        skill.observation_count = 5;
        skill.tested_contexts = 3;

        assert!(skill.is_reliable());
        assert!(!skill.needs_validation());
    }

    #[test]
    fn test_query_matching() {
        let skill = SkillsEntity::new(
            "Error Handling",
            "Handle database errors",
            SkillType::ErrorSolution,
            "try { ... } catch (e) { ... }",
        )
        .with_tags(vec!["database".into(), "exception".into()]);

        assert!(skill.matches_query("error"));
        assert!(skill.matches_query("database"));
        assert!(skill.matches_query("catch"));
        assert!(!skill.matches_query("network"));
    }

    #[test]
    fn test_skill_type() {
        assert_eq!(SkillType::ErrorSolution.as_str(), "error_solution");
        assert_eq!(SkillType::Technique.as_str(), "technique");
    }
}
