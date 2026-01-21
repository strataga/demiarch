//! Skill data types and models
//!
//! This module defines the core types for the learned skills system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A learned skill extracted from agent interactions
///
/// Skills represent reusable patterns, techniques, or knowledge that can be
/// applied to similar future tasks. They are extracted automatically from
/// successful agent executions and stored for retrieval during planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedSkill {
    /// Unique identifier for the skill
    pub id: String,
    /// Human-readable name for the skill
    pub name: String,
    /// Brief description of what the skill does
    pub description: String,
    /// The skill category for organization
    pub category: SkillCategory,
    /// The pattern or template that can be reused
    pub pattern: SkillPattern,
    /// How the skill was learned (source context)
    pub source: SkillSource,
    /// Confidence level in the skill's applicability
    pub confidence: SkillConfidence,
    /// Tags for searching and filtering
    pub tags: Vec<String>,
    /// Usage statistics
    pub usage_stats: SkillUsageStats,
    /// Additional metadata
    pub metadata: SkillMetadata,
    /// When the skill was created
    pub created_at: DateTime<Utc>,
    /// When the skill was last updated
    pub updated_at: DateTime<Utc>,
}

impl LearnedSkill {
    /// Create a new learned skill with the given name and description
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: SkillCategory,
        pattern: SkillPattern,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            category,
            pattern,
            source: SkillSource::default(),
            confidence: SkillConfidence::Medium,
            tags: Vec::new(),
            usage_stats: SkillUsageStats::default(),
            metadata: SkillMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the source context for this skill
    pub fn with_source(mut self, source: SkillSource) -> Self {
        self.source = source;
        self
    }

    /// Set the confidence level
    pub fn with_confidence(mut self, confidence: SkillConfidence) -> Self {
        self.confidence = confidence;
        self
    }

    /// Add tags to the skill
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: SkillMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Record that the skill was used
    pub fn record_usage(&mut self, success: bool) {
        self.usage_stats.times_used += 1;
        if success {
            self.usage_stats.success_count += 1;
        } else {
            self.usage_stats.failure_count += 1;
        }
        self.usage_stats.last_used_at = Some(Utc::now());
        self.updated_at = Utc::now();

        // Adjust confidence based on success rate
        self.confidence = self.calculate_confidence();
    }

    /// Calculate confidence based on usage statistics
    fn calculate_confidence(&self) -> SkillConfidence {
        if self.usage_stats.times_used < 3 {
            return self.confidence;
        }

        let success_rate = self.usage_stats.success_rate();
        if success_rate >= 0.9 {
            SkillConfidence::High
        } else if success_rate >= 0.7 {
            SkillConfidence::Medium
        } else {
            SkillConfidence::Low
        }
    }

    /// Check if this skill matches given tags
    pub fn matches_tags(&self, query_tags: &[String]) -> bool {
        query_tags.iter().any(|qt| {
            self.tags.iter().any(|t| t.to_lowercase().contains(&qt.to_lowercase()))
        })
    }

    /// Check if this skill matches a text query
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
            || self.description.to_lowercase().contains(&query_lower)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            || self.category.as_str().to_lowercase().contains(&query_lower)
    }
}

/// Categories for organizing skills
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    /// Code generation patterns
    CodeGeneration,
    /// Code refactoring techniques
    Refactoring,
    /// Testing strategies
    Testing,
    /// Debugging approaches
    Debugging,
    /// Architecture patterns
    Architecture,
    /// Performance optimization
    Performance,
    /// Security best practices
    Security,
    /// Documentation patterns
    Documentation,
    /// API design patterns
    ApiDesign,
    /// Database patterns
    Database,
    /// Error handling
    ErrorHandling,
    /// Other/uncategorized
    Other,
}

impl SkillCategory {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CodeGeneration => "code_generation",
            Self::Refactoring => "refactoring",
            Self::Testing => "testing",
            Self::Debugging => "debugging",
            Self::Architecture => "architecture",
            Self::Performance => "performance",
            Self::Security => "security",
            Self::Documentation => "documentation",
            Self::ApiDesign => "api_design",
            Self::Database => "database",
            Self::ErrorHandling => "error_handling",
            Self::Other => "other",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "code_generation" | "codegen" => Self::CodeGeneration,
            "refactoring" | "refactor" => Self::Refactoring,
            "testing" | "test" => Self::Testing,
            "debugging" | "debug" => Self::Debugging,
            "architecture" | "arch" => Self::Architecture,
            "performance" | "perf" => Self::Performance,
            "security" | "sec" => Self::Security,
            "documentation" | "docs" => Self::Documentation,
            "api_design" | "api" => Self::ApiDesign,
            "database" | "db" => Self::Database,
            "error_handling" | "errors" => Self::ErrorHandling,
            _ => Self::Other,
        }
    }

    /// Get all categories
    pub fn all() -> &'static [SkillCategory] {
        &[
            Self::CodeGeneration,
            Self::Refactoring,
            Self::Testing,
            Self::Debugging,
            Self::Architecture,
            Self::Performance,
            Self::Security,
            Self::Documentation,
            Self::ApiDesign,
            Self::Database,
            Self::ErrorHandling,
            Self::Other,
        ]
    }
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The reusable pattern or template from a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPattern {
    /// The pattern type
    pub pattern_type: PatternType,
    /// Template or example code/text that can be adapted
    pub template: String,
    /// Variables or placeholders in the template
    pub variables: Vec<PatternVariable>,
    /// Conditions when this pattern is applicable
    pub applicability: Vec<String>,
    /// Known limitations or edge cases
    pub limitations: Vec<String>,
}

impl SkillPattern {
    /// Create a new code pattern
    pub fn code(template: impl Into<String>) -> Self {
        Self {
            pattern_type: PatternType::CodeTemplate,
            template: template.into(),
            variables: Vec::new(),
            applicability: Vec::new(),
            limitations: Vec::new(),
        }
    }

    /// Create a new technique pattern
    pub fn technique(description: impl Into<String>) -> Self {
        Self {
            pattern_type: PatternType::Technique,
            template: description.into(),
            variables: Vec::new(),
            applicability: Vec::new(),
            limitations: Vec::new(),
        }
    }

    /// Create a new architecture pattern
    pub fn architecture(description: impl Into<String>) -> Self {
        Self {
            pattern_type: PatternType::ArchitecturePattern,
            template: description.into(),
            variables: Vec::new(),
            applicability: Vec::new(),
            limitations: Vec::new(),
        }
    }

    /// Add variables to the pattern
    pub fn with_variables(mut self, variables: Vec<PatternVariable>) -> Self {
        self.variables = variables;
        self
    }

    /// Add applicability conditions
    pub fn with_applicability(mut self, conditions: Vec<String>) -> Self {
        self.applicability = conditions;
        self
    }

    /// Add known limitations
    pub fn with_limitations(mut self, limitations: Vec<String>) -> Self {
        self.limitations = limitations;
        self
    }
}

/// Types of patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Code template that can be adapted
    CodeTemplate,
    /// A technique or approach (not necessarily code)
    Technique,
    /// Architecture or design pattern
    ArchitecturePattern,
    /// Command or script template
    CommandTemplate,
    /// Configuration pattern
    ConfigPattern,
    /// Workflow or process pattern
    WorkflowPattern,
}

impl PatternType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CodeTemplate => "code_template",
            Self::Technique => "technique",
            Self::ArchitecturePattern => "architecture_pattern",
            Self::CommandTemplate => "command_template",
            Self::ConfigPattern => "config_pattern",
            Self::WorkflowPattern => "workflow_pattern",
        }
    }
}

/// A variable or placeholder in a pattern template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternVariable {
    /// Name of the variable
    pub name: String,
    /// Description of what it represents
    pub description: String,
    /// Example value
    pub example: Option<String>,
    /// Whether the variable is required
    pub required: bool,
}

impl PatternVariable {
    /// Create a new required variable
    pub fn required(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            example: None,
            required: true,
        }
    }

    /// Create a new optional variable
    pub fn optional(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            example: None,
            required: false,
        }
    }

    /// Add an example value
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }
}

/// Source context for where a skill was learned
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillSource {
    /// Project ID where the skill was extracted
    pub project_id: Option<String>,
    /// Feature ID that produced the skill
    pub feature_id: Option<String>,
    /// Agent type that executed the task
    pub agent_type: Option<String>,
    /// Original task description
    pub original_task: Option<String>,
    /// Model used for extraction
    pub model_used: Option<String>,
    /// Tokens used during extraction
    pub tokens_used: Option<u32>,
}

impl SkillSource {
    /// Create a new skill source
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the project ID
    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Set the feature ID
    pub fn with_feature(mut self, feature_id: impl Into<String>) -> Self {
        self.feature_id = Some(feature_id.into());
        self
    }

    /// Set the agent type
    pub fn with_agent(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = Some(agent_type.into());
        self
    }

    /// Set the original task
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.original_task = Some(task.into());
        self
    }

    /// Set the model used
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_used = Some(model.into());
        self
    }

    /// Set tokens used
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }
}

/// Confidence level for a skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillConfidence {
    /// Low confidence - skill may not be widely applicable
    Low,
    /// Medium confidence - skill works in tested scenarios
    Medium,
    /// High confidence - skill has been validated multiple times
    High,
}

impl SkillConfidence {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            _ => Self::Medium,
        }
    }

    /// Get a numeric score for ranking
    pub fn score(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
        }
    }
}

impl std::fmt::Display for SkillConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Usage statistics for a skill
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillUsageStats {
    /// Total times the skill has been used
    pub times_used: u32,
    /// Successful applications
    pub success_count: u32,
    /// Failed applications
    pub failure_count: u32,
    /// When the skill was last used
    pub last_used_at: Option<DateTime<Utc>>,
}

impl SkillUsageStats {
    /// Calculate the success rate
    pub fn success_rate(&self) -> f64 {
        if self.times_used == 0 {
            return 0.0;
        }
        self.success_count as f64 / self.times_used as f64
    }
}

/// Additional metadata for a skill
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Programming language this skill applies to (if specific)
    pub language: Option<String>,
    /// Framework this skill applies to (if specific)
    pub framework: Option<String>,
    /// Related skills (by ID)
    pub related_skills: Vec<String>,
    /// Prerequisites (skills needed before using this one)
    pub prerequisites: Vec<String>,
    /// Version of the skill (for updates)
    pub version: u32,
    /// Whether this skill is deprecated
    pub deprecated: bool,
    /// Reason for deprecation
    pub deprecation_reason: Option<String>,
    /// Custom key-value metadata
    pub custom: serde_json::Value,
}

impl SkillMetadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the programming language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set the framework
    pub fn with_framework(mut self, framework: impl Into<String>) -> Self {
        self.framework = Some(framework.into());
        self
    }

    /// Add related skills
    pub fn with_related(mut self, skill_ids: Vec<String>) -> Self {
        self.related_skills = skill_ids;
        self
    }

    /// Add prerequisites
    pub fn with_prerequisites(mut self, skill_ids: Vec<String>) -> Self {
        self.prerequisites = skill_ids;
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self, reason: impl Into<String>) -> Self {
        self.deprecated = true;
        self.deprecation_reason = Some(reason.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learned_skill_creation() {
        let skill = LearnedSkill::new(
            "Error Handling Pattern",
            "Wrap operations in Result with context",
            SkillCategory::ErrorHandling,
            SkillPattern::code("fn do_thing() -> Result<T, Error> { ... }"),
        );

        assert!(!skill.id.is_empty());
        assert_eq!(skill.name, "Error Handling Pattern");
        assert_eq!(skill.category, SkillCategory::ErrorHandling);
        assert_eq!(skill.confidence, SkillConfidence::Medium);
    }

    #[test]
    fn test_skill_usage_tracking() {
        let mut skill = LearnedSkill::new(
            "Test Skill",
            "A test skill",
            SkillCategory::Testing,
            SkillPattern::technique("Write tests first"),
        );

        skill.record_usage(true);
        skill.record_usage(true);
        skill.record_usage(false);

        assert_eq!(skill.usage_stats.times_used, 3);
        assert_eq!(skill.usage_stats.success_count, 2);
        assert_eq!(skill.usage_stats.failure_count, 1);
        assert!(skill.usage_stats.last_used_at.is_some());
    }

    #[test]
    fn test_skill_confidence_adjustment() {
        let mut skill = LearnedSkill::new(
            "High Success Skill",
            "A skill with high success rate",
            SkillCategory::CodeGeneration,
            SkillPattern::code("// code"),
        );

        // Record many successes
        for _ in 0..10 {
            skill.record_usage(true);
        }

        assert_eq!(skill.confidence, SkillConfidence::High);
    }

    #[test]
    fn test_skill_matching() {
        let skill = LearnedSkill::new(
            "Rust Error Handling",
            "Pattern for handling errors in Rust",
            SkillCategory::ErrorHandling,
            SkillPattern::code("// code"),
        )
        .with_tags(vec!["rust".into(), "errors".into(), "result".into()]);

        assert!(skill.matches_query("rust"));
        assert!(skill.matches_query("error"));
        assert!(skill.matches_query("handling"));
        assert!(!skill.matches_query("python"));

        assert!(skill.matches_tags(&["rust".into()]));
        assert!(skill.matches_tags(&["errors".into()]));
        assert!(!skill.matches_tags(&["python".into()]));
    }

    #[test]
    fn test_skill_category_parsing() {
        assert_eq!(SkillCategory::from_str("code_generation"), SkillCategory::CodeGeneration);
        assert_eq!(SkillCategory::from_str("codegen"), SkillCategory::CodeGeneration);
        assert_eq!(SkillCategory::from_str("testing"), SkillCategory::Testing);
        assert_eq!(SkillCategory::from_str("unknown"), SkillCategory::Other);
    }

    #[test]
    fn test_skill_confidence_ordering() {
        assert!(SkillConfidence::Low < SkillConfidence::Medium);
        assert!(SkillConfidence::Medium < SkillConfidence::High);
    }

    #[test]
    fn test_pattern_variable() {
        let var = PatternVariable::required("type_name", "The name of the type")
            .with_example("MyStruct");

        assert!(var.required);
        assert_eq!(var.example, Some("MyStruct".into()));
    }

    #[test]
    fn test_skill_source() {
        let source = SkillSource::new()
            .with_project("proj-123")
            .with_feature("feat-456")
            .with_agent("Coder")
            .with_task("Implement error handling")
            .with_model("claude-3")
            .with_tokens(1500);

        assert_eq!(source.project_id, Some("proj-123".into()));
        assert_eq!(source.feature_id, Some("feat-456".into()));
        assert_eq!(source.agent_type, Some("Coder".into()));
        assert_eq!(source.tokens_used, Some(1500));
    }

    #[test]
    fn test_skill_metadata() {
        let metadata = SkillMetadata::new()
            .with_language("Rust")
            .with_framework("Actix")
            .with_related(vec!["skill-1".into(), "skill-2".into()])
            .deprecated("Use v2 instead");

        assert_eq!(metadata.language, Some("Rust".into()));
        assert_eq!(metadata.framework, Some("Actix".into()));
        assert!(metadata.deprecated);
        assert_eq!(metadata.deprecation_reason, Some("Use v2 instead".into()));
    }

    #[test]
    fn test_usage_stats_success_rate() {
        let mut stats = SkillUsageStats::default();
        assert_eq!(stats.success_rate(), 0.0);

        stats.times_used = 10;
        stats.success_count = 8;
        stats.failure_count = 2;

        assert!((stats.success_rate() - 0.8).abs() < 0.001);
    }
}
