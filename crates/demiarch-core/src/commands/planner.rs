//! Phase planner service for LLM-based feature decomposition
//!
//! This module provides AI-powered phase planning and feature extraction.

use crate::commands::feature::{Feature, FeatureRepository, FeatureStatus};
use crate::commands::phase::{Phase, PhaseRepository};
use crate::llm::{LlmClient, Message};
use crate::storage::Database;
use crate::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Planning configuration
#[derive(Debug, Clone)]
pub struct PlanningConfig {
    /// Model to use for planning
    pub model: String,
    /// Temperature for LLM responses
    pub temperature: f32,
    /// Maximum tokens for response
    pub max_tokens: u32,
}

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
        }
    }
}

/// A planned feature extracted from requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedFeature {
    /// Feature name/title
    pub name: String,
    /// Feature description
    pub description: String,
    /// Acceptance criteria
    pub acceptance_criteria: Vec<String>,
    /// Target phase name
    pub phase: String,
    /// Priority (1-5)
    pub priority: i32,
    /// Labels/tags
    pub labels: Vec<String>,
}

/// The result of phase planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningResult {
    /// Project summary
    pub summary: String,
    /// Planned phases
    pub phases: Vec<PlannedPhase>,
    /// Tokens used for planning
    pub tokens_used: Option<u32>,
    /// Model used
    pub model: String,
}

/// A planned phase with its features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedPhase {
    /// Phase name
    pub name: String,
    /// Phase description
    pub description: String,
    /// Features in this phase
    pub features: Vec<PlannedFeature>,
}

/// LLM response structure for phase planning
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmPlanningResponse {
    summary: String,
    phases: Vec<LlmPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmPhase {
    name: String,
    description: String,
    features: Vec<LlmFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmFeature {
    name: String,
    description: String,
    acceptance_criteria: Vec<String>,
    priority: i32,
    labels: Vec<String>,
}

const PLANNING_SYSTEM_PROMPT: &str = r#"You are an expert software architect and project planner. Your task is to analyze project requirements and break them down into organized phases with user stories.

RULES:
1. Use the standard 4-phase structure: Discovery, Planning, Building, Complete
2. Each feature should have clear acceptance criteria in Given/When/Then format
3. Priority ranges from 1 (highest) to 5 (lowest)
4. Labels should categorize features (e.g., "ui", "api", "database", "auth", "testing")
5. Features in "Discovery" phase are for research and requirements gathering
6. Features in "Planning" phase are for technical design decisions
7. Features in "Building" phase are for implementation
8. Features in "Complete" phase are for deployment and documentation

OUTPUT FORMAT:
You must respond with valid JSON only, no markdown formatting. The structure must be:
{
  "summary": "Brief project summary",
  "phases": [
    {
      "name": "Phase Name",
      "description": "Phase description",
      "features": [
        {
          "name": "Feature name",
          "description": "Feature description",
          "acceptance_criteria": [
            "Given X When Y Then Z",
            "Given A When B Then C"
          ],
          "priority": 1,
          "labels": ["ui", "auth"]
        }
      ]
    }
  ]
}
"#;

/// Phase planner service
pub struct PhasePlanner<'a> {
    db: &'a Database,
    llm_client: &'a LlmClient,
    config: PlanningConfig,
}

impl<'a> PhasePlanner<'a> {
    /// Create a new phase planner
    pub fn new(db: &'a Database, llm_client: &'a LlmClient) -> Self {
        Self {
            db,
            llm_client,
            config: PlanningConfig::default(),
        }
    }

    /// Create a new phase planner with custom config
    pub fn with_config(
        db: &'a Database,
        llm_client: &'a LlmClient,
        config: PlanningConfig,
    ) -> Self {
        Self {
            db,
            llm_client,
            config,
        }
    }

    /// Plan phases and features for a project based on a description
    pub async fn plan_from_description(
        &self,
        _project_id: &str,
        description: &str,
    ) -> Result<PlanningResult> {
        // Generate the plan using LLM
        let messages = vec![
            Message::system(PLANNING_SYSTEM_PROMPT),
            Message::user(format!(
                "Please analyze the following project requirements and create a comprehensive phase breakdown with user stories:\n\n{}",
                description
            )),
        ];

        let response = self
            .llm_client
            .complete(messages, Some(&self.config.model))
            .await?;

        // Parse the LLM response
        let planning_response = self.parse_llm_response(&response.content)?;

        // Convert to PlanningResult
        let result = PlanningResult {
            summary: planning_response.summary,
            phases: planning_response
                .phases
                .into_iter()
                .map(|p| PlannedPhase {
                    name: p.name.clone(),
                    description: p.description,
                    features: p
                        .features
                        .into_iter()
                        .map(|f| PlannedFeature {
                            name: f.name,
                            description: f.description,
                            acceptance_criteria: f.acceptance_criteria,
                            phase: p.name.clone(),
                            priority: f.priority.clamp(1, 5),
                            labels: f.labels,
                        })
                        .collect(),
                })
                .collect(),
            tokens_used: Some(response.tokens_used),
            model: self.config.model.clone(),
        };

        Ok(result)
    }

    /// Apply a planning result to a project (create phases and features in DB)
    pub async fn apply_plan(
        &self,
        project_id: &str,
        plan: &PlanningResult,
        conversation_id: Option<&str>,
    ) -> Result<AppliedPlan> {
        let phase_repo = PhaseRepository::new(self.db);
        let feature_repo = FeatureRepository::new(self.db);

        let mut phases_created = 0;
        let mut features_created = 0;
        let mut phase_ids = std::collections::HashMap::new();

        // Create or get phases
        for (order_index, planned_phase) in plan.phases.iter().enumerate() {
            // Check if phase already exists
            let existing = phase_repo
                .get_by_name(project_id, &planned_phase.name)
                .await?;

            let phase_id = if let Some(existing_phase) = existing {
                existing_phase.id
            } else {
                let phase = Phase::new(project_id, &planned_phase.name, order_index as i32)
                    .with_description(&planned_phase.description);
                phase_repo.create(&phase).await?;
                phases_created += 1;
                phase.id
            };

            phase_ids.insert(planned_phase.name.clone(), phase_id.clone());

            // Create features for this phase
            for planned_feature in &planned_phase.features {
                let criteria = planned_feature.acceptance_criteria.join("\n");
                let feature = Feature::new(project_id, &planned_feature.name)
                    .with_description(&planned_feature.description)
                    .with_acceptance_criteria(criteria)
                    .with_labels(planned_feature.labels.clone())
                    .with_phase(&phase_id)
                    .with_priority(planned_feature.priority)
                    .with_status(FeatureStatus::Backlog);

                feature_repo.create(&feature).await?;
                features_created += 1;
            }
        }

        // Record the extraction history
        let extraction_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO feature_extraction_history
            (id, project_id, conversation_id, model_used, tokens_used, phases_created, features_created, raw_response, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&extraction_id)
        .bind(project_id)
        .bind(conversation_id)
        .bind(&plan.model)
        .bind(plan.tokens_used.map(|t| t as i64))
        .bind(phases_created as i64)
        .bind(features_created as i64)
        .bind(serde_json::to_string(plan).ok())
        .bind(Utc::now())
        .execute(self.db.pool())
        .await?;

        Ok(AppliedPlan {
            extraction_id,
            phases_created,
            features_created,
        })
    }

    /// Plan and apply in one step
    pub async fn plan_and_apply(
        &self,
        project_id: &str,
        description: &str,
        conversation_id: Option<&str>,
    ) -> Result<(PlanningResult, AppliedPlan)> {
        let plan = self.plan_from_description(project_id, description).await?;
        let applied = self.apply_plan(project_id, &plan, conversation_id).await?;
        Ok((plan, applied))
    }

    /// Parse LLM response into structured format
    fn parse_llm_response(&self, content: &str) -> Result<LlmPlanningResponse> {
        // Try to extract JSON from the response (in case it's wrapped in markdown)
        let json_content = if content.contains("```json") {
            content
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(content)
                .trim()
        } else if content.contains("```") {
            content
                .split("```")
                .nth(1)
                .unwrap_or(content)
                .trim()
        } else {
            content.trim()
        };

        serde_json::from_str(json_content).map_err(|e| {
            crate::Error::Parse(format!(
                "Failed to parse LLM planning response: {}. Content: {}",
                e,
                &json_content[..json_content.len().min(500)]
            ))
        })
    }
}

/// Result of applying a plan to a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedPlan {
    /// ID of the extraction history record
    pub extraction_id: String,
    /// Number of phases created
    pub phases_created: usize,
    /// Number of features created
    pub features_created: usize,
}

// ============================================================================
// Feature extraction from conversation
// ============================================================================

const FEATURE_EXTRACTION_PROMPT: &str = r#"You are an expert at extracting features and user stories from conversations.

Analyze the conversation and extract any features that have been discussed or implied.

For each feature, provide:
1. A clear name/title
2. A description of what it does
3. Acceptance criteria in Given/When/Then format
4. Suggested priority (1=highest to 5=lowest)
5. Relevant labels/tags

OUTPUT FORMAT:
Respond with valid JSON only:
{
  "features": [
    {
      "name": "Feature name",
      "description": "Description",
      "acceptance_criteria": ["Given X When Y Then Z"],
      "phase": "Building",
      "priority": 2,
      "labels": ["label1", "label2"]
    }
  ]
}
"#;

/// Extract features from a conversation
pub async fn extract_features_from_conversation(
    _db: &Database,
    llm_client: &LlmClient,
    conversation_history: &[String],
    _project_id: &str,
) -> Result<Vec<PlannedFeature>> {
    let conversation_text = conversation_history.join("\n\n---\n\n");

    let messages = vec![
        Message::system(FEATURE_EXTRACTION_PROMPT),
        Message::user(format!(
            "Extract features from this conversation:\n\n{}",
            conversation_text
        )),
    ];

    let response = llm_client.complete(messages, None).await?;

    // Parse response
    let content = response.content.trim();
    let json_content = if content.contains("```json") {
        content
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(content)
            .trim()
    } else {
        content
    };

    #[derive(Deserialize)]
    struct ExtractedFeatures {
        features: Vec<LlmFeature>,
    }

    let extracted: ExtractedFeatures = serde_json::from_str(json_content).map_err(|e| {
        crate::Error::Parse(format!("Failed to parse feature extraction response: {}", e))
    })?;

    Ok(extracted
        .features
        .into_iter()
        .map(|f| PlannedFeature {
            name: f.name,
            description: f.description,
            acceptance_criteria: f.acceptance_criteria,
            phase: "Building".to_string(), // Default to Building phase
            priority: f.priority.clamp(1, 5),
            labels: f.labels,
        })
        .collect())
}

// ============================================================================
// Helper functions for chat integration
// ============================================================================

/// Check if a message is requesting phase planning
pub fn is_planning_request(message: &str) -> bool {
    let planning_keywords = [
        "break down",
        "breakdown",
        "plan out",
        "plan the",
        "create phases",
        "phase planning",
        "user stories",
        "acceptance criteria",
        "organize features",
        "feature breakdown",
        "project structure",
        "implementation plan",
    ];

    let lower = message.to_lowercase();
    planning_keywords
        .iter()
        .any(|keyword| lower.contains(keyword))
}

/// Format a planning result for display
pub fn format_planning_result(result: &PlanningResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("## Project Summary\n\n{}\n\n", result.summary));

    for phase in &result.phases {
        output.push_str(&format!(
            "### {} ({})\n\n{}\n\n",
            phase.name,
            phase.features.len(),
            phase.description
        ));

        for (i, feature) in phase.features.iter().enumerate() {
            output.push_str(&format!(
                "**{}. {}** (P{})\n",
                i + 1,
                feature.name,
                feature.priority
            ));
            output.push_str(&format!("{}\n\n", feature.description));

            if !feature.acceptance_criteria.is_empty() {
                output.push_str("Acceptance Criteria:\n");
                for ac in &feature.acceptance_criteria {
                    output.push_str(&format!("- {}\n", ac));
                }
                output.push('\n');
            }

            if !feature.labels.is_empty() {
                output.push_str(&format!("Labels: {}\n\n", feature.labels.join(", ")));
            }
        }
    }

    if let Some(tokens) = result.tokens_used {
        output.push_str(&format!("\n---\n*Tokens used: {}*\n", tokens));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_planning_request() {
        assert!(is_planning_request("Can you break down this project?"));
        assert!(is_planning_request("Please create phases for the app"));
        assert!(is_planning_request("I need user stories for this feature"));
        assert!(!is_planning_request("Hello, how are you?"));
        assert!(!is_planning_request("Show me the code"));
    }

    #[test]
    fn test_parse_planning_response() {
        let json = r#"{
            "summary": "Test project",
            "phases": [
                {
                    "name": "Building",
                    "description": "Implementation",
                    "features": [
                        {
                            "name": "Feature 1",
                            "description": "Test feature",
                            "acceptance_criteria": ["Given X When Y Then Z"],
                            "priority": 2,
                            "labels": ["ui"]
                        }
                    ]
                }
            ]
        }"#;

        let response: LlmPlanningResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.summary, "Test project");
        assert_eq!(response.phases.len(), 1);
        assert_eq!(response.phases[0].features.len(), 1);
    }

    #[test]
    fn test_format_planning_result() {
        let result = PlanningResult {
            summary: "A test project".to_string(),
            phases: vec![PlannedPhase {
                name: "Building".to_string(),
                description: "Implementation phase".to_string(),
                features: vec![PlannedFeature {
                    name: "Feature 1".to_string(),
                    description: "Test feature".to_string(),
                    acceptance_criteria: vec!["Given X When Y Then Z".to_string()],
                    phase: "Building".to_string(),
                    priority: 2,
                    labels: vec!["ui".to_string()],
                }],
            }],
            tokens_used: Some(100),
            model: "test-model".to_string(),
        };

        let formatted = format_planning_result(&result);
        assert!(formatted.contains("A test project"));
        assert!(formatted.contains("Building"));
        assert!(formatted.contains("Feature 1"));
        assert!(formatted.contains("Given X When Y Then Z"));
    }
}
