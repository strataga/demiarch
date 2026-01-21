//! Skill extraction from agent results
//!
//! This module implements LLM-assisted skill extraction from successful
//! agent executions. It analyzes artifacts and outputs to identify
//! reusable patterns that can benefit future tasks.

use std::sync::Arc;

use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::agents::traits::{AgentArtifact, AgentResult, ArtifactType};
use crate::error::Result;
use crate::llm::{LlmClient, Message};

use super::types::{
    LearnedSkill, PatternType, PatternVariable, SkillCategory, SkillConfidence, SkillMetadata,
    SkillPattern, SkillSource,
};

/// Minimum confidence score from LLM to create a skill
const MIN_EXTRACTION_CONFIDENCE: f32 = 0.6;

/// Maximum number of skills to extract from a single result
const MAX_SKILLS_PER_RESULT: usize = 3;

/// Skill extractor using LLM-assisted pattern recognition
///
/// The extractor analyzes agent results and artifacts to identify
/// reusable patterns and techniques that can be stored as skills.
#[derive(Clone)]
pub struct SkillExtractor {
    /// LLM client for pattern analysis
    llm_client: Arc<LlmClient>,
    /// Whether to extract from code artifacts
    extract_from_code: bool,
    /// Whether to extract from review artifacts
    extract_from_reviews: bool,
    /// Whether to extract from test artifacts
    extract_from_tests: bool,
    /// Minimum artifact size to consider (in bytes)
    min_artifact_size: usize,
}

impl SkillExtractor {
    /// Create a new skill extractor with the given LLM client
    pub fn new(llm_client: Arc<LlmClient>) -> Self {
        Self {
            llm_client,
            extract_from_code: true,
            extract_from_reviews: true,
            extract_from_tests: true,
            min_artifact_size: 50,
        }
    }

    /// Configure whether to extract from code artifacts
    pub fn with_code_extraction(mut self, enabled: bool) -> Self {
        self.extract_from_code = enabled;
        self
    }

    /// Configure whether to extract from review artifacts
    pub fn with_review_extraction(mut self, enabled: bool) -> Self {
        self.extract_from_reviews = enabled;
        self
    }

    /// Configure whether to extract from test artifacts
    pub fn with_test_extraction(mut self, enabled: bool) -> Self {
        self.extract_from_tests = enabled;
        self
    }

    /// Set minimum artifact size for extraction
    pub fn with_min_artifact_size(mut self, size: usize) -> Self {
        self.min_artifact_size = size;
        self
    }

    /// Extract skills from an agent result
    ///
    /// Analyzes the result and its artifacts to identify reusable patterns.
    /// Returns a list of learned skills extracted from the result.
    pub async fn extract_from_result(
        &self,
        result: &AgentResult,
        context: ExtractionContext,
    ) -> Result<Vec<LearnedSkill>> {
        // Only extract from successful results
        if !result.success {
            debug!("Skipping skill extraction for failed result");
            return Ok(Vec::new());
        }

        let mut skills = Vec::new();

        // Filter artifacts by type and size
        let eligible_artifacts: Vec<_> = result
            .artifacts
            .iter()
            .filter(|a| self.is_eligible_artifact(a))
            .collect();

        if eligible_artifacts.is_empty() {
            debug!("No eligible artifacts for skill extraction");
            return Ok(Vec::new());
        }

        info!(
            artifact_count = eligible_artifacts.len(),
            "Extracting skills from agent result"
        );

        // Analyze artifacts for skills
        for artifact in eligible_artifacts {
            match self.analyze_artifact(artifact, &context).await {
                Ok(mut extracted) => {
                    skills.append(&mut extracted);
                    if skills.len() >= MAX_SKILLS_PER_RESULT {
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to analyze artifact for skills");
                }
            }
        }

        // Limit total skills
        skills.truncate(MAX_SKILLS_PER_RESULT);

        info!(skill_count = skills.len(), "Skills extracted from result");

        Ok(skills)
    }

    /// Check if an artifact is eligible for skill extraction
    fn is_eligible_artifact(&self, artifact: &AgentArtifact) -> bool {
        if artifact.content.len() < self.min_artifact_size {
            return false;
        }

        match artifact.artifact_type {
            ArtifactType::Code => self.extract_from_code,
            ArtifactType::Review => self.extract_from_reviews,
            ArtifactType::Test => self.extract_from_tests,
            ArtifactType::Plan => true, // Always extract from plans
            _ => false,
        }
    }

    /// Analyze a single artifact for skills
    async fn analyze_artifact(
        &self,
        artifact: &AgentArtifact,
        context: &ExtractionContext,
    ) -> Result<Vec<LearnedSkill>> {
        let prompt = self.build_extraction_prompt(artifact, context);

        let messages = vec![
            Message::system(EXTRACTION_SYSTEM_PROMPT),
            Message::user(&prompt),
        ];

        let response = self.llm_client.complete(messages, None).await?;

        // Parse the LLM response to extract skills
        let extracted = self.parse_extraction_response(&response.content, artifact, context)?;

        Ok(extracted)
    }

    /// Build the extraction prompt for an artifact
    fn build_extraction_prompt(&self, artifact: &AgentArtifact, context: &ExtractionContext) -> String {
        let artifact_type = match artifact.artifact_type {
            ArtifactType::Code => "code",
            ArtifactType::Review => "code review",
            ArtifactType::Test => "test code",
            ArtifactType::Plan => "execution plan",
            _ => "artifact",
        };

        format!(
            r#"Analyze this {artifact_type} for reusable skills and patterns.

Task Context:
{task_context}

Artifact Name: {name}
Artifact Content:
```
{content}
```

Identify any reusable patterns, techniques, or approaches that could benefit future similar tasks.
For each skill found, provide:
1. A clear, descriptive name
2. A concise description of what it does
3. The category (code_generation, refactoring, testing, debugging, architecture, performance, security, documentation, api_design, database, error_handling, or other)
4. The pattern template that can be reused
5. Any variables or placeholders in the pattern
6. When this pattern is applicable
7. Known limitations
8. A confidence score (0.0-1.0) for how reusable this pattern is

Return your analysis as JSON with the following structure:
{{
    "skills": [
        {{
            "name": "Skill Name",
            "description": "What this skill does",
            "category": "category_name",
            "pattern_type": "code_template|technique|architecture_pattern",
            "template": "The reusable template or pattern",
            "variables": [
                {{"name": "var_name", "description": "what it represents", "required": true}}
            ],
            "applicability": ["When this applies", "Another condition"],
            "limitations": ["Known limitation"],
            "confidence": 0.8
        }}
    ]
}}"#,
            artifact_type = artifact_type,
            task_context = context.task_description.as_deref().unwrap_or("General task"),
            name = artifact.name,
            content = truncate_content(&artifact.content, 4000),
        )
    }

    /// Parse the LLM extraction response
    fn parse_extraction_response(
        &self,
        response: &str,
        artifact: &AgentArtifact,
        context: &ExtractionContext,
    ) -> Result<Vec<LearnedSkill>> {
        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response);

        let extraction: ExtractionResponse = serde_json::from_str(&json_str).map_err(|e| {
            warn!(error = %e, "Failed to parse extraction response as JSON");
            crate::error::Error::Parse(format!("Invalid extraction response: {}", e))
        })?;

        let skills: Vec<LearnedSkill> = extraction
            .skills
            .into_iter()
            .filter(|s| s.confidence >= MIN_EXTRACTION_CONFIDENCE)
            .map(|s| self.convert_extracted_skill(s, artifact, context))
            .collect();

        Ok(skills)
    }

    /// Convert an extracted skill response to a LearnedSkill
    fn convert_extracted_skill(
        &self,
        extracted: ExtractedSkill,
        artifact: &AgentArtifact,
        context: &ExtractionContext,
    ) -> LearnedSkill {
        let category = SkillCategory::from_str(&extracted.category);

        let pattern_type = match extracted.pattern_type.as_str() {
            "technique" => PatternType::Technique,
            "architecture_pattern" => PatternType::ArchitecturePattern,
            "command_template" => PatternType::CommandTemplate,
            "config_pattern" => PatternType::ConfigPattern,
            "workflow_pattern" => PatternType::WorkflowPattern,
            _ => PatternType::CodeTemplate,
        };

        let variables: Vec<PatternVariable> = extracted
            .variables
            .into_iter()
            .map(|v| {
                if v.required {
                    PatternVariable::required(v.name, v.description)
                } else {
                    PatternVariable::optional(v.name, v.description)
                }
            })
            .collect();

        let pattern = SkillPattern {
            pattern_type,
            template: extracted.template,
            variables,
            applicability: extracted.applicability,
            limitations: extracted.limitations,
        };

        let confidence = if extracted.confidence >= 0.8 {
            SkillConfidence::High
        } else if extracted.confidence >= 0.6 {
            SkillConfidence::Medium
        } else {
            SkillConfidence::Low
        };

        let source = SkillSource::new()
            .with_project(context.project_id.clone().unwrap_or_default())
            .with_feature(context.feature_id.clone().unwrap_or_default())
            .with_agent(context.agent_type.clone().unwrap_or_default())
            .with_task(context.task_description.clone().unwrap_or_default())
            .with_model(self.llm_client.default_model().to_string());

        let mut metadata = SkillMetadata::new();
        if let Some(lang) = detect_language(&artifact.content) {
            metadata = metadata.with_language(lang);
        }

        LearnedSkill::new(extracted.name, extracted.description, category, pattern)
            .with_source(source)
            .with_confidence(confidence)
            .with_metadata(metadata)
    }
}

/// Context for skill extraction
#[derive(Debug, Clone, Default)]
pub struct ExtractionContext {
    /// Project ID where the work occurred
    pub project_id: Option<String>,
    /// Feature ID that was implemented
    pub feature_id: Option<String>,
    /// Type of agent that produced the result
    pub agent_type: Option<String>,
    /// Description of the task
    pub task_description: Option<String>,
}

impl ExtractionContext {
    /// Create a new extraction context
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

    /// Set the task description
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.task_description = Some(task.into());
        self
    }
}

/// Response structure from LLM extraction
#[derive(Debug, Deserialize)]
struct ExtractionResponse {
    skills: Vec<ExtractedSkill>,
}

/// A skill extracted from the LLM response
#[derive(Debug, Deserialize)]
struct ExtractedSkill {
    name: String,
    description: String,
    category: String,
    #[serde(default = "default_pattern_type")]
    pattern_type: String,
    template: String,
    #[serde(default)]
    variables: Vec<ExtractedVariable>,
    #[serde(default)]
    applicability: Vec<String>,
    #[serde(default)]
    limitations: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f32,
}

fn default_pattern_type() -> String {
    "code_template".to_string()
}

fn default_confidence() -> f32 {
    0.7
}

/// A variable in an extracted skill
#[derive(Debug, Deserialize)]
struct ExtractedVariable {
    name: String,
    description: String,
    #[serde(default)]
    required: bool,
}

/// System prompt for skill extraction
const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an expert software engineer tasked with identifying reusable patterns and techniques from code artifacts.

Your goal is to extract skills that:
1. Are genuinely reusable across different contexts
2. Capture non-obvious techniques or patterns
3. Can be adapted with minimal changes
4. Provide clear value for similar future tasks

Focus on:
- Design patterns and architectural approaches
- Error handling techniques
- Testing strategies
- Performance optimizations
- Security best practices
- Code organization patterns

Avoid extracting:
- Project-specific implementations
- Trivial or obvious patterns
- Code that requires significant context to understand
- Patterns that are too generic to be useful

Return your analysis as valid JSON only, with no additional text or explanation."#;

/// Truncate content to a maximum length
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated = &content[..max_len];
        format!("{}...\n[Content truncated]", truncated)
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
        // Skip language identifier if present
        let json_start = if let Some(newline) = response[potential_start..].find('\n') {
            potential_start + newline + 1
        } else {
            potential_start
        };
        if let Some(end) = response[json_start..].find("```") {
            return response[json_start..json_start + end].trim().to_string();
        }
    }

    // Try to find raw JSON object
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }

    // Return as-is if no JSON found
    response.to_string()
}

/// Detect programming language from content
fn detect_language(content: &str) -> Option<String> {
    let content_lower = content.to_lowercase();

    // Rust indicators
    if content.contains("fn ") && (content.contains("->") || content.contains("impl ")) {
        return Some("rust".to_string());
    }

    // Python indicators
    if content.contains("def ") && content.contains(":") && !content.contains("{") {
        return Some("python".to_string());
    }

    // TypeScript/JavaScript indicators
    if content.contains("const ") || content.contains("let ") || content.contains("function ") {
        if content.contains(": ") && (content.contains("interface ") || content.contains(": string")
            || content.contains(": number"))
        {
            return Some("typescript".to_string());
        }
        return Some("javascript".to_string());
    }

    // Go indicators
    if content.contains("func ") && content.contains("package ") {
        return Some("go".to_string());
    }

    // Java/Kotlin indicators
    if content_lower.contains("public class ") || content_lower.contains("private class ") {
        return Some("java".to_string());
    }

    // Ruby indicators
    if content.contains("def ") && content.contains("end") && !content.contains("{") {
        return Some("ruby".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_context() {
        let ctx = ExtractionContext::new()
            .with_project("proj-123")
            .with_feature("feat-456")
            .with_agent("Coder")
            .with_task("Implement error handling");

        assert_eq!(ctx.project_id, Some("proj-123".into()));
        assert_eq!(ctx.feature_id, Some("feat-456".into()));
        assert_eq!(ctx.agent_type, Some("Coder".into()));
        assert_eq!(ctx.task_description, Some("Implement error handling".into()));
    }

    #[test]
    fn test_truncate_content() {
        let short = "Short content";
        assert_eq!(truncate_content(short, 100), short);

        let long = "x".repeat(100);
        let truncated = truncate_content(&long, 50);
        assert!(truncated.len() < 100);
        assert!(truncated.contains("[Content truncated]"));
    }

    #[test]
    fn test_extract_json_from_response() {
        // JSON in code block
        let response = "Here's the analysis:\n```json\n{\"skills\": []}\n```";
        assert_eq!(extract_json_from_response(response), "{\"skills\": []}");

        // Raw JSON
        let response = "The result is {\"skills\": []} as shown.";
        assert_eq!(extract_json_from_response(response), "{\"skills\": []}");

        // Just JSON
        let response = "{\"skills\": []}";
        assert_eq!(extract_json_from_response(response), "{\"skills\": []}");
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(
            detect_language("fn main() -> Result<()> { Ok(()) }"),
            Some("rust".into())
        );
        assert_eq!(
            detect_language("def hello():\n    print('hi')"),
            Some("python".into())
        );
        assert_eq!(
            detect_language("const x: string = 'hello';"),
            Some("typescript".into())
        );
        assert_eq!(
            detect_language("const x = 'hello';"),
            Some("javascript".into())
        );
        assert_eq!(
            detect_language("func main() { package main }"),
            Some("go".into())
        );
        assert_eq!(detect_language("plain text"), None);
    }

    #[test]
    fn test_parse_extraction_response() {
        let json = r#"{
            "skills": [
                {
                    "name": "Error Wrapping",
                    "description": "Wrap errors with context",
                    "category": "error_handling",
                    "pattern_type": "code_template",
                    "template": "anyhow::Context::context(result, \"message\")",
                    "variables": [],
                    "applicability": ["When using anyhow"],
                    "limitations": [],
                    "confidence": 0.85
                }
            ]
        }"#;

        let response: ExtractionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.skills.len(), 1);
        assert_eq!(response.skills[0].name, "Error Wrapping");
        assert_eq!(response.skills[0].confidence, 0.85);
    }
}
