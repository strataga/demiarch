//! Knowledge relationships for the GraphRAG system
//!
//! This module defines the relationship (edge) types for the knowledge graph.
//! Relationships connect entities and describe how they relate to each other.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A relationship between two knowledge entities
///
/// Relationships are edges in the knowledge graph, connecting entities
/// and describing how they relate. They have a weight indicating strength
/// and evidence explaining why the relationship was inferred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRelationship {
    /// Unique identifier for the relationship
    pub id: String,
    /// ID of the source entity
    pub source_entity_id: String,
    /// ID of the target entity
    pub target_entity_id: String,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Weight/strength of the relationship (0.0 to 1.0)
    pub weight: f32,
    /// Evidence supporting this relationship
    pub evidence: Vec<RelationshipEvidence>,
    /// When the relationship was created
    pub created_at: DateTime<Utc>,
    /// When the relationship was last updated
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeRelationship {
    /// Create a new relationship between two entities
    pub fn new(
        source_entity_id: impl Into<String>,
        target_entity_id: impl Into<String>,
        relationship_type: RelationshipType,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4().to_string(),
            source_entity_id: source_entity_id.into(),
            target_entity_id: target_entity_id.into(),
            relationship_type,
            weight: 0.5,
            evidence: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Add evidence for the relationship
    pub fn with_evidence(mut self, evidence: Vec<RelationshipEvidence>) -> Self {
        self.evidence = evidence;
        self
    }

    /// Add a single piece of evidence
    pub fn add_evidence(&mut self, evidence: RelationshipEvidence) {
        self.evidence.push(evidence);
        self.updated_at = Utc::now();
        // Boost weight when evidence is added
        self.weight = (self.weight + 0.1).min(1.0);
    }

    /// Update the weight (clamped to 0.0-1.0)
    pub fn update_weight(&mut self, delta: f32) {
        self.weight = (self.weight + delta).clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    /// Check if this relationship is bidirectional
    pub fn is_bidirectional(&self) -> bool {
        matches!(
            self.relationship_type,
            RelationshipType::SimilarTo | RelationshipType::RelatedTo | RelationshipType::ConflictsWith
        )
    }

    /// Get the inverse relationship type (if applicable)
    pub fn inverse_type(&self) -> Option<RelationshipType> {
        match self.relationship_type {
            RelationshipType::Uses => Some(RelationshipType::UsedBy),
            RelationshipType::UsedBy => Some(RelationshipType::Uses),
            RelationshipType::DependsOn => Some(RelationshipType::DependencyOf),
            RelationshipType::DependencyOf => Some(RelationshipType::DependsOn),
            RelationshipType::PrerequisiteFor => Some(RelationshipType::Requires),
            RelationshipType::Requires => Some(RelationshipType::PrerequisiteFor),
            RelationshipType::PartOf => Some(RelationshipType::Contains),
            RelationshipType::Contains => Some(RelationshipType::PartOf),
            RelationshipType::ImplementedBy => Some(RelationshipType::Implements),
            RelationshipType::Implements => Some(RelationshipType::ImplementedBy),
            // Bidirectional relationships have no inverse
            RelationshipType::SimilarTo
            | RelationshipType::RelatedTo
            | RelationshipType::ConflictsWith
            | RelationshipType::AppliesTo => None,
        }
    }
}

/// Types of relationships between entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Source uses target (e.g., "tokio uses async-trait")
    Uses,
    /// Source is used by target (inverse of Uses)
    UsedBy,
    /// Source depends on target (stronger than Uses)
    DependsOn,
    /// Source is a dependency of target (inverse of DependsOn)
    DependencyOf,
    /// Source is similar to target (bidirectional)
    SimilarTo,
    /// Source is a prerequisite for target
    PrerequisiteFor,
    /// Source requires target (inverse of PrerequisiteFor)
    Requires,
    /// Source applies to/is applicable to target
    AppliesTo,
    /// Source is part of target
    PartOf,
    /// Source contains target (inverse of PartOf)
    Contains,
    /// Source is implemented by target
    ImplementedBy,
    /// Source implements target (inverse of ImplementedBy)
    Implements,
    /// Source conflicts with target (bidirectional)
    ConflictsWith,
    /// Source is related to target (generic, bidirectional)
    RelatedTo,
}

impl RelationshipType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uses => "uses",
            Self::UsedBy => "used_by",
            Self::DependsOn => "depends_on",
            Self::DependencyOf => "dependency_of",
            Self::SimilarTo => "similar_to",
            Self::PrerequisiteFor => "prerequisite_for",
            Self::Requires => "requires",
            Self::AppliesTo => "applies_to",
            Self::PartOf => "part_of",
            Self::Contains => "contains",
            Self::ImplementedBy => "implemented_by",
            Self::Implements => "implements",
            Self::ConflictsWith => "conflicts_with",
            Self::RelatedTo => "related_to",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "uses" => Some(Self::Uses),
            "used_by" | "usedby" => Some(Self::UsedBy),
            "depends_on" | "dependson" => Some(Self::DependsOn),
            "dependency_of" | "dependencyof" => Some(Self::DependencyOf),
            "similar_to" | "similarto" => Some(Self::SimilarTo),
            "prerequisite_for" | "prerequisitefor" => Some(Self::PrerequisiteFor),
            "requires" => Some(Self::Requires),
            "applies_to" | "appliesto" => Some(Self::AppliesTo),
            "part_of" | "partof" => Some(Self::PartOf),
            "contains" => Some(Self::Contains),
            "implemented_by" | "implementedby" => Some(Self::ImplementedBy),
            "implements" => Some(Self::Implements),
            "conflicts_with" | "conflictswith" => Some(Self::ConflictsWith),
            "related_to" | "relatedto" => Some(Self::RelatedTo),
            _ => None,
        }
    }

    /// Get all relationship types
    pub fn all() -> &'static [RelationshipType] {
        &[
            Self::Uses,
            Self::UsedBy,
            Self::DependsOn,
            Self::DependencyOf,
            Self::SimilarTo,
            Self::PrerequisiteFor,
            Self::Requires,
            Self::AppliesTo,
            Self::PartOf,
            Self::Contains,
            Self::ImplementedBy,
            Self::Implements,
            Self::ConflictsWith,
            Self::RelatedTo,
        ]
    }

    /// Check if this relationship type is bidirectional
    pub fn is_bidirectional(&self) -> bool {
        matches!(
            self,
            Self::SimilarTo | Self::RelatedTo | Self::ConflictsWith
        )
    }

    /// Get the inverse of this relationship type (if applicable)
    ///
    /// Returns `None` for bidirectional relationships or those without
    /// a natural inverse.
    pub fn inverse_type(&self) -> Option<Self> {
        match self {
            Self::Uses => Some(Self::UsedBy),
            Self::UsedBy => Some(Self::Uses),
            Self::DependsOn => Some(Self::DependencyOf),
            Self::DependencyOf => Some(Self::DependsOn),
            Self::PrerequisiteFor => Some(Self::Requires),
            Self::Requires => Some(Self::PrerequisiteFor),
            Self::PartOf => Some(Self::Contains),
            Self::Contains => Some(Self::PartOf),
            Self::ImplementedBy => Some(Self::Implements),
            Self::Implements => Some(Self::ImplementedBy),
            // Bidirectional relationships have no inverse
            Self::SimilarTo | Self::RelatedTo | Self::ConflictsWith | Self::AppliesTo => None,
        }
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Evidence supporting a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvidence {
    /// Source of the evidence
    pub source: EvidenceSource,
    /// Description of the evidence
    pub description: String,
    /// Confidence in this evidence (0.0 to 1.0)
    pub confidence: f32,
    /// When the evidence was added
    pub timestamp: DateTime<Utc>,
}

impl RelationshipEvidence {
    /// Create new evidence from co-occurrence in a skill
    pub fn from_skill_cooccurrence(skill_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            source: EvidenceSource::SkillCooccurrence {
                skill_id: skill_id.into(),
            },
            description: description.into(),
            confidence: 0.6,
            timestamp: Utc::now(),
        }
    }

    /// Create new evidence from LLM inference
    pub fn from_llm_inference(model: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            source: EvidenceSource::LlmInference {
                model: model.into(),
            },
            description: description.into(),
            confidence: 0.7,
            timestamp: Utc::now(),
        }
    }

    /// Create new evidence from explicit user input
    pub fn from_user_input(description: impl Into<String>) -> Self {
        Self {
            source: EvidenceSource::UserInput,
            description: description.into(),
            confidence: 0.9,
            timestamp: Utc::now(),
        }
    }

    /// Set the confidence level
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Source of relationship evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    /// Entities co-occurred in the same skill
    SkillCooccurrence { skill_id: String },
    /// Relationship was inferred by LLM
    LlmInference { model: String },
    /// Relationship was explicitly stated by user
    UserInput,
    /// Relationship was inferred from usage patterns
    UsagePattern,
    /// Relationship was imported from external source
    External { source: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_creation() {
        let rel = KnowledgeRelationship::new("entity-1", "entity-2", RelationshipType::Uses)
            .with_weight(0.8);

        assert!(!rel.id.is_empty());
        assert_eq!(rel.source_entity_id, "entity-1");
        assert_eq!(rel.target_entity_id, "entity-2");
        assert_eq!(rel.relationship_type, RelationshipType::Uses);
        assert_eq!(rel.weight, 0.8);
    }

    #[test]
    fn test_add_evidence() {
        let mut rel = KnowledgeRelationship::new("e1", "e2", RelationshipType::DependsOn);
        let initial_weight = rel.weight;

        rel.add_evidence(RelationshipEvidence::from_skill_cooccurrence(
            "skill-123",
            "Both entities appear in error handling skill",
        ));

        assert_eq!(rel.evidence.len(), 1);
        assert!(rel.weight > initial_weight);
    }

    #[test]
    fn test_weight_clamping() {
        let mut rel = KnowledgeRelationship::new("e1", "e2", RelationshipType::SimilarTo);

        rel.update_weight(2.0);
        assert_eq!(rel.weight, 1.0);

        rel.update_weight(-3.0);
        assert_eq!(rel.weight, 0.0);
    }

    #[test]
    fn test_bidirectional_check() {
        let rel1 = KnowledgeRelationship::new("e1", "e2", RelationshipType::SimilarTo);
        let rel2 = KnowledgeRelationship::new("e1", "e2", RelationshipType::Uses);

        assert!(rel1.is_bidirectional());
        assert!(!rel2.is_bidirectional());
    }

    #[test]
    fn test_inverse_type() {
        assert_eq!(
            RelationshipType::Uses.inverse_type(),
            Some(RelationshipType::UsedBy)
        );
        assert_eq!(
            RelationshipType::DependsOn.inverse_type(),
            Some(RelationshipType::DependencyOf)
        );
        assert_eq!(RelationshipType::SimilarTo.inverse_type(), None);
    }

    #[test]
    fn test_relationship_type_parsing() {
        assert_eq!(
            RelationshipType::parse("uses"),
            Some(RelationshipType::Uses)
        );
        assert_eq!(
            RelationshipType::parse("depends_on"),
            Some(RelationshipType::DependsOn)
        );
        assert_eq!(
            RelationshipType::parse("SIMILAR_TO"),
            Some(RelationshipType::SimilarTo)
        );
        assert_eq!(RelationshipType::parse("unknown"), None);
    }

    #[test]
    fn test_evidence_creation() {
        let evidence = RelationshipEvidence::from_llm_inference(
            "gpt-4",
            "Inferred from documentation analysis",
        )
        .with_confidence(0.85);

        assert_eq!(evidence.confidence, 0.85);
        assert!(matches!(
            evidence.source,
            EvidenceSource::LlmInference { .. }
        ));
    }
}
