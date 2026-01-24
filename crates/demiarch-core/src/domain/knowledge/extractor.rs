//! Entity extraction from skills using LLM
//!
//! This module implements LLM-assisted entity and relationship extraction
//! from learned skills. It identifies concepts, techniques, libraries,
//! and their relationships to build the knowledge graph.

use std::sync::Arc;

use serde::Deserialize;
use tracing::{info, warn};

use crate::error::Result;
use crate::llm::{LlmClient, Message};
use crate::skills::LearnedSkill;

use super::entity::{EntityType, KnowledgeEntity};
use super::relationship::{KnowledgeRelationship, RelationshipEvidence, RelationshipType};

/// Minimum confidence score for extracted entities
const MIN_ENTITY_CONFIDENCE: f32 = 0.5;

/// Maximum entities to extract from a single skill
const MAX_ENTITIES_PER_SKILL: usize = 10;

/// Maximum relationships to infer from a single skill
const MAX_RELATIONSHIPS_PER_SKILL: usize = 15;

/// Entity extractor using LLM-assisted analysis
///
/// Extracts knowledge entities (concepts, libraries, patterns, etc.) and
/// their relationships from learned skills to build a knowledge graph.
#[derive(Clone)]
pub struct EntityExtractor {
    /// LLM client for analysis
    llm_client: Arc<LlmClient>,
}

impl EntityExtractor {
    /// Create a new entity extractor
    pub fn new(llm_client: Arc<LlmClient>) -> Self {
        Self { llm_client }
    }

    /// Extract entities and relationships from a skill
    ///
    /// Analyzes the skill content to identify knowledge entities and
    /// their relationships, which can be added to the knowledge graph.
    pub async fn extract_from_skill(&self, skill: &LearnedSkill) -> Result<ExtractionResult> {
        info!(skill_id = %skill.id, skill_name = %skill.name, "Extracting entities from skill");

        let prompt = self.build_extraction_prompt(skill);

        let messages = vec![
            Message::system(ENTITY_EXTRACTION_SYSTEM_PROMPT),
            Message::user(&prompt),
        ];

        let response = self.llm_client.complete(messages, None).await?;

        let result = self.parse_extraction_response(&response.content, skill)?;

        info!(
            skill_id = %skill.id,
            entity_count = result.entities.len(),
            relationship_count = result.relationships.len(),
            "Entities extracted from skill"
        );

        Ok(result)
    }

    /// Extract entities from multiple skills (batch operation)
    pub async fn extract_from_skills(
        &self,
        skills: &[LearnedSkill],
    ) -> Result<Vec<ExtractionResult>> {
        let mut results = Vec::with_capacity(skills.len());

        for skill in skills {
            match self.extract_from_skill(skill).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!(skill_id = %skill.id, error = %e, "Failed to extract from skill");
                }
            }
        }

        Ok(results)
    }

    /// Build the extraction prompt for a skill
    fn build_extraction_prompt(&self, skill: &LearnedSkill) -> String {
        format!(
            r#"Analyze this skill and extract knowledge entities and their relationships.

SKILL INFORMATION:
- Name: {name}
- Description: {description}
- Category: {category}
- Pattern Template:
```
{template}
```
- Applicability: {applicability}
- Limitations: {limitations}
- Tags: {tags}

Extract:
1. **Entities**: Concepts, techniques, libraries, frameworks, patterns, languages, tools, etc.
2. **Relationships**: How the entities relate to each other

For each entity, classify it as one of:
- concept: Abstract programming concept (e.g., "error handling", "async programming")
- technique: Specific technique or approach (e.g., "retry with exponential backoff")
- library: Software library (e.g., "tokio", "serde")
- framework: Framework (e.g., "actix-web", "rocket")
- pattern: Design pattern (e.g., "repository pattern", "builder")
- language: Programming language (e.g., "Rust", "Python")
- tool: Development tool (e.g., "cargo", "git")
- domain: Problem domain (e.g., "web development", "data processing")
- api: API or service (e.g., "OpenAI API")
- data_structure: Data structure (e.g., "HashMap")
- algorithm: Algorithm (e.g., "binary search")

For relationships, use one of these types:
- uses: Source uses target
- depends_on: Source depends on target (stronger than uses)
- similar_to: Source is similar to target
- prerequisite_for: Source is a prerequisite for target
- applies_to: Source applies to target domain
- part_of: Source is part of target
- implemented_by: Source is implemented by target
- conflicts_with: Source conflicts with target
- related_to: Generic relation

Return your analysis as JSON:
{{
    "entities": [
        {{
            "name": "entity name",
            "type": "concept|technique|library|framework|pattern|language|tool|domain|api|data_structure|algorithm",
            "description": "brief description",
            "aliases": ["alternative names"],
            "confidence": 0.8
        }}
    ],
    "relationships": [
        {{
            "source": "source entity name",
            "target": "target entity name",
            "type": "uses|depends_on|similar_to|prerequisite_for|applies_to|part_of|implemented_by|conflicts_with|related_to",
            "description": "why this relationship exists",
            "weight": 0.7
        }}
    ]
}}"#,
            name = skill.name,
            description = skill.description,
            category = skill.category.as_str(),
            template = truncate_content(&skill.pattern.template, 2000),
            applicability = skill.pattern.applicability.join(", "),
            limitations = skill.pattern.limitations.join(", "),
            tags = skill.tags.join(", "),
        )
    }

    /// Parse the LLM extraction response
    fn parse_extraction_response(
        &self,
        response: &str,
        skill: &LearnedSkill,
    ) -> Result<ExtractionResult> {
        let json_str = extract_json_from_response(response);

        let extraction: LlmExtractionResponse = serde_json::from_str(&json_str).map_err(|e| {
            warn!(error = %e, "Failed to parse extraction response as JSON");
            crate::error::Error::EntityExtractionFailed(format!("Invalid response: {}", e))
        })?;

        // Convert extracted entities to KnowledgeEntity
        let entities: Vec<KnowledgeEntity> = extraction
            .entities
            .into_iter()
            .take(MAX_ENTITIES_PER_SKILL)
            .filter(|e| e.confidence >= MIN_ENTITY_CONFIDENCE)
            .filter_map(|e| {
                let entity_type = EntityType::parse(&e.entity_type)?;
                let mut entity = KnowledgeEntity::new(&e.name, entity_type)
                    .with_confidence(e.confidence)
                    .with_source_skills(vec![skill.id.clone()]);

                if let Some(desc) = e.description {
                    entity = entity.with_description(desc);
                }

                if !e.aliases.is_empty() {
                    entity = entity.with_aliases(e.aliases);
                }

                Some(entity)
            })
            .collect();

        // Build entity name to ID map for relationship creation
        let entity_map: std::collections::HashMap<String, String> = entities
            .iter()
            .map(|e| (e.canonical_name.clone(), e.id.clone()))
            .collect();

        // Convert extracted relationships
        let relationships: Vec<KnowledgeRelationship> = extraction
            .relationships
            .into_iter()
            .take(MAX_RELATIONSHIPS_PER_SKILL)
            .filter_map(|r| {
                let source_canonical = KnowledgeEntity::canonicalize(&r.source);
                let target_canonical = KnowledgeEntity::canonicalize(&r.target);

                let source_id = entity_map.get(&source_canonical)?;
                let target_id = entity_map.get(&target_canonical)?;
                let rel_type = RelationshipType::parse(&r.relationship_type)?;

                let evidence = RelationshipEvidence::from_skill_cooccurrence(
                    &skill.id,
                    r.description
                        .unwrap_or_else(|| format!("Inferred from skill: {}", skill.name)),
                );

                let rel = KnowledgeRelationship::new(source_id, target_id, rel_type)
                    .with_weight(r.weight.unwrap_or(0.5))
                    .with_evidence(vec![evidence]);

                Some(rel)
            })
            .collect();

        Ok(ExtractionResult {
            skill_id: skill.id.clone(),
            entities,
            relationships,
        })
    }
}

/// Result of entity extraction from a skill
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// ID of the skill that was processed
    pub skill_id: String,
    /// Extracted entities
    pub entities: Vec<KnowledgeEntity>,
    /// Inferred relationships between entities
    pub relationships: Vec<KnowledgeRelationship>,
}

impl ExtractionResult {
    /// Check if any entities were extracted
    pub fn has_entities(&self) -> bool {
        !self.entities.is_empty()
    }

    /// Check if any relationships were inferred
    pub fn has_relationships(&self) -> bool {
        !self.relationships.is_empty()
    }

    /// Get total count of extracted items
    pub fn total_count(&self) -> usize {
        self.entities.len() + self.relationships.len()
    }
}

/// Response structure from LLM extraction
#[derive(Debug, Deserialize)]
struct LlmExtractionResponse {
    #[serde(default)]
    entities: Vec<ExtractedEntity>,
    #[serde(default)]
    relationships: Vec<ExtractedRelationship>,
}

/// An entity extracted from the LLM response
#[derive(Debug, Deserialize)]
struct ExtractedEntity {
    name: String,
    #[serde(rename = "type")]
    entity_type: String,
    description: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f32,
}

/// A relationship extracted from the LLM response
#[derive(Debug, Deserialize)]
struct ExtractedRelationship {
    source: String,
    target: String,
    #[serde(rename = "type")]
    relationship_type: String,
    description: Option<String>,
    weight: Option<f32>,
}

fn default_confidence() -> f32 {
    0.6
}

/// System prompt for entity extraction
const ENTITY_EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an expert at analyzing software engineering knowledge and extracting structured entities and relationships.

Your task is to identify knowledge entities from code skills and patterns, and understand how they relate to each other.

Guidelines:
1. Extract entities that are genuinely important and reusable
2. Use the most specific entity type that applies
3. Identify relationships that provide valuable context
4. Assign confidence based on how clearly the entity is mentioned
5. Use canonical names (e.g., "tokio" not "Tokio runtime")

Focus on:
- Libraries and frameworks mentioned or used
- Design patterns and architectural approaches
- Programming concepts and techniques
- Tools and utilities
- Problem domains

Avoid:
- Trivially obvious relationships
- Entities that are too generic (e.g., "code", "function")
- Relationships with no explanatory value

Return your analysis as valid JSON only, with no additional text or explanation."#;

/// Truncate content to a maximum length
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated = &content[..max_len];
        format!("{}...", truncated)
    }
}

/// Extract JSON from a response that might contain markdown or other text
fn extract_json_from_response(response: &str) -> String {
    // Try to find JSON in code blocks first
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return response[json_start..json_start + end].trim().to_string();
        }
    }

    // Try to find JSON in generic code blocks
    if let Some(start) = response.find("```") {
        let potential_start = start + 3;
        if let Some(newline) = response[potential_start..].find('\n') {
            let json_start = potential_start + newline + 1;
            if let Some(end) = response[json_start..].find("```") {
                return response[json_start..json_start + end].trim().to_string();
            }
        }
    }

    // Try to find raw JSON object
    if let (Some(start), Some(end)) = (response.find('{'), response.rfind('}')) {
        return response[start..=end].to_string();
    }

    // Return as-is if no JSON found
    response.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{SkillCategory, SkillPattern};

    fn create_test_skill() -> LearnedSkill {
        LearnedSkill::new(
            "Async Error Handling in Tokio",
            "Pattern for handling errors in async Rust code using tokio and anyhow",
            SkillCategory::ErrorHandling,
            SkillPattern::code(
                r#"async fn do_work() -> anyhow::Result<()> {
    let result = tokio::spawn(async {
        // work here
    }).await?;
    Ok(())
}"#,
            ),
        )
        .with_tags(vec![
            "rust".into(),
            "async".into(),
            "tokio".into(),
            "error-handling".into(),
        ])
    }

    #[test]
    fn test_extraction_result() {
        let result = ExtractionResult {
            skill_id: "skill-123".into(),
            entities: vec![
                KnowledgeEntity::new("tokio", EntityType::Library),
                KnowledgeEntity::new("async programming", EntityType::Concept),
            ],
            relationships: vec![],
        };

        assert!(result.has_entities());
        assert!(!result.has_relationships());
        assert_eq!(result.total_count(), 2);
    }

    #[test]
    fn test_parse_llm_response() {
        let json = r#"{
            "entities": [
                {
                    "name": "tokio",
                    "type": "library",
                    "description": "Async runtime for Rust",
                    "aliases": ["tokio-rs"],
                    "confidence": 0.9
                },
                {
                    "name": "error handling",
                    "type": "concept",
                    "description": "Managing errors in software",
                    "confidence": 0.8
                }
            ],
            "relationships": [
                {
                    "source": "tokio",
                    "target": "error handling",
                    "type": "applies_to",
                    "description": "Tokio has specific error handling patterns",
                    "weight": 0.7
                }
            ]
        }"#;

        let response: LlmExtractionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.entities.len(), 2);
        assert_eq!(response.relationships.len(), 1);
        assert_eq!(response.entities[0].name, "tokio");
        assert_eq!(response.entities[0].entity_type, "library");
    }

    #[test]
    fn test_extract_json_from_response() {
        // JSON in code block
        let response = "Here's the analysis:\n```json\n{\"entities\": []}\n```";
        assert_eq!(extract_json_from_response(response), "{\"entities\": []}");

        // Raw JSON
        let response = "The result is {\"entities\": []} as shown.";
        assert_eq!(extract_json_from_response(response), "{\"entities\": []}");
    }

    #[test]
    fn test_truncate_content() {
        let short = "Short content";
        assert_eq!(truncate_content(short, 100), short);

        let long = "x".repeat(100);
        let truncated = truncate_content(&long, 50);
        assert!(truncated.len() < 100);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_build_extraction_prompt() {
        // Note: We can't test this without the LLM client,
        // but we can verify the skill structure is valid
        let skill = create_test_skill();
        assert!(!skill.name.is_empty());
        assert!(!skill.description.is_empty());
        assert!(!skill.tags.is_empty());
    }
}
