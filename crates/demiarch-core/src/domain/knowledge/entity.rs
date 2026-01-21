//! Knowledge entity types for the GraphRAG system
//!
//! This module defines the core entity types for the knowledge graph.
//! Entities represent concepts, techniques, libraries, and other knowledge
//! extracted from skills.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A knowledge entity representing a concept, technique, library, etc.
///
/// Entities are nodes in the knowledge graph, extracted from learned skills
/// through LLM-based entity extraction. Each entity has a canonical name
/// for deduplication and can have multiple aliases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntity {
    /// Unique identifier for the entity
    pub id: String,
    /// Type of knowledge entity
    pub entity_type: EntityType,
    /// Human-readable name for the entity
    pub name: String,
    /// Normalized name for deduplication (lowercase, no special chars)
    pub canonical_name: String,
    /// Optional description of the entity
    pub description: Option<String>,
    /// Alternative names or spellings
    pub aliases: Vec<String>,
    /// IDs of skills that contributed to this entity
    pub source_skill_ids: Vec<String>,
    /// Confidence score (0.0 to 1.0) based on extraction and usage
    pub confidence: f32,
    /// When the entity was created
    pub created_at: DateTime<Utc>,
    /// When the entity was last updated
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeEntity {
    /// Create a new knowledge entity
    pub fn new(name: impl Into<String>, entity_type: EntityType) -> Self {
        let name = name.into();
        let canonical_name = Self::canonicalize(&name);
        let now = Utc::now();

        Self {
            id: Uuid::new_v4().to_string(),
            entity_type,
            name,
            canonical_name,
            description: None,
            aliases: Vec::new(),
            source_skill_ids: Vec::new(),
            confidence: 0.5,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add aliases
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// Set source skill IDs
    pub fn with_source_skills(mut self, skill_ids: Vec<String>) -> Self {
        self.source_skill_ids = skill_ids;
        self
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add a source skill ID
    pub fn add_source_skill(&mut self, skill_id: String) {
        if !self.source_skill_ids.contains(&skill_id) {
            self.source_skill_ids.push(skill_id);
            self.updated_at = Utc::now();
        }
    }

    /// Add an alias
    pub fn add_alias(&mut self, alias: String) {
        let canonical_alias = Self::canonicalize(&alias);
        if !self.aliases.iter().any(|a| Self::canonicalize(a) == canonical_alias) {
            self.aliases.push(alias);
            self.updated_at = Utc::now();
        }
    }

    /// Update confidence score (clamped to 0.0-1.0)
    pub fn update_confidence(&mut self, delta: f32) {
        self.confidence = (self.confidence + delta).clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    /// Canonicalize a name for deduplication
    ///
    /// Converts to lowercase, removes special characters, and normalizes whitespace
    pub fn canonicalize(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Check if this entity matches a name (including aliases)
    pub fn matches_name(&self, query: &str) -> bool {
        let canonical_query = Self::canonicalize(query);
        if self.canonical_name == canonical_query {
            return true;
        }
        self.aliases
            .iter()
            .any(|alias| Self::canonicalize(alias) == canonical_query)
    }
}

/// Types of knowledge entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Abstract concept (e.g., "error handling", "async programming")
    Concept,
    /// Specific technique (e.g., "retry with exponential backoff")
    Technique,
    /// Software library (e.g., "tokio", "serde")
    Library,
    /// Framework (e.g., "actix-web", "rocket")
    Framework,
    /// Design pattern (e.g., "repository pattern", "builder")
    Pattern,
    /// Programming language (e.g., "Rust", "Python")
    Language,
    /// Development tool (e.g., "cargo", "git")
    Tool,
    /// Problem domain (e.g., "web development", "data processing")
    Domain,
    /// API or service (e.g., "OpenAI API", "GitHub API")
    Api,
    /// Data structure (e.g., "HashMap", "BTreeMap")
    DataStructure,
    /// Algorithm (e.g., "binary search", "quicksort")
    Algorithm,
}

impl EntityType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Concept => "concept",
            Self::Technique => "technique",
            Self::Library => "library",
            Self::Framework => "framework",
            Self::Pattern => "pattern",
            Self::Language => "language",
            Self::Tool => "tool",
            Self::Domain => "domain",
            Self::Api => "api",
            Self::DataStructure => "data_structure",
            Self::Algorithm => "algorithm",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "concept" => Some(Self::Concept),
            "technique" => Some(Self::Technique),
            "library" | "lib" => Some(Self::Library),
            "framework" => Some(Self::Framework),
            "pattern" => Some(Self::Pattern),
            "language" | "lang" => Some(Self::Language),
            "tool" => Some(Self::Tool),
            "domain" => Some(Self::Domain),
            "api" | "service" => Some(Self::Api),
            "data_structure" | "datastructure" => Some(Self::DataStructure),
            "algorithm" | "algo" => Some(Self::Algorithm),
            _ => None,
        }
    }

    /// Get all entity types
    pub fn all() -> &'static [EntityType] {
        &[
            Self::Concept,
            Self::Technique,
            Self::Library,
            Self::Framework,
            Self::Pattern,
            Self::Language,
            Self::Tool,
            Self::Domain,
            Self::Api,
            Self::DataStructure,
            Self::Algorithm,
        ]
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = KnowledgeEntity::new("tokio", EntityType::Library)
            .with_description("Async runtime for Rust")
            .with_confidence(0.8);

        assert!(!entity.id.is_empty());
        assert_eq!(entity.name, "tokio");
        assert_eq!(entity.canonical_name, "tokio");
        assert_eq!(entity.entity_type, EntityType::Library);
        assert_eq!(entity.confidence, 0.8);
    }

    #[test]
    fn test_canonicalization() {
        assert_eq!(KnowledgeEntity::canonicalize("Tokio"), "tokio");
        assert_eq!(
            KnowledgeEntity::canonicalize("async-std"),
            "asyncstd"
        );
        assert_eq!(
            KnowledgeEntity::canonicalize("Error Handling"),
            "error handling"
        );
        assert_eq!(
            KnowledgeEntity::canonicalize("  Multiple   Spaces  "),
            "multiple spaces"
        );
    }

    #[test]
    fn test_matches_name() {
        let mut entity = KnowledgeEntity::new("async-std", EntityType::Library);
        entity.add_alias("async_std".into());
        entity.add_alias("Async-Std".into());

        assert!(entity.matches_name("async-std"));
        assert!(entity.matches_name("async_std"));
        assert!(entity.matches_name("Async-Std"));
        assert!(entity.matches_name("asyncstd")); // Canonical form
        assert!(!entity.matches_name("tokio"));
    }

    #[test]
    fn test_add_source_skill() {
        let mut entity = KnowledgeEntity::new("test", EntityType::Concept);

        entity.add_source_skill("skill-1".into());
        entity.add_source_skill("skill-2".into());
        entity.add_source_skill("skill-1".into()); // Duplicate

        assert_eq!(entity.source_skill_ids.len(), 2);
    }

    #[test]
    fn test_confidence_clamping() {
        let mut entity = KnowledgeEntity::new("test", EntityType::Concept);

        entity.update_confidence(2.0);
        assert_eq!(entity.confidence, 1.0);

        entity.update_confidence(-3.0);
        assert_eq!(entity.confidence, 0.0);
    }

    #[test]
    fn test_entity_type_parsing() {
        assert_eq!(EntityType::parse("library"), Some(EntityType::Library));
        assert_eq!(EntityType::parse("lib"), Some(EntityType::Library));
        assert_eq!(EntityType::parse("CONCEPT"), Some(EntityType::Concept));
        assert_eq!(EntityType::parse("unknown"), None);
    }
}
