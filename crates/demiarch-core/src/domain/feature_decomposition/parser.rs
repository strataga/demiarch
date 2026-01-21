//! Plan parsing utilities
//!
//! Parses LLM responses into execution plans.

use serde_json;

use super::task::{ExecutionPlan, PlanTask};

/// Parser for converting LLM responses into execution plans
pub struct PlanParser;

impl PlanParser {
    /// Create a new plan parser
    pub fn new() -> Self {
        Self
    }

    /// Parse an LLM response into an execution plan
    ///
    /// Tries JSON parsing first, falls back to heuristic parsing
    pub fn parse(&self, original_task: &str, response: &str) -> ExecutionPlan {
        // Try JSON parsing first
        if let Some(plan) = self.try_parse_json(response) {
            return plan;
        }

        // Fall back to heuristic parsing
        self.parse_heuristic(original_task, response)
    }

    /// Try to parse JSON from the response
    fn try_parse_json(&self, response: &str) -> Option<ExecutionPlan> {
        // Find JSON in the response
        let json_start = response.find('{')?;
        let json_end = response.rfind('}')?;

        if json_end < json_start {
            return None;
        }

        let json_str = &response[json_start..=json_end];
        serde_json::from_str::<ExecutionPlan>(json_str).ok()
    }

    /// Parse using heuristics when JSON isn't available
    fn parse_heuristic(&self, original_task: &str, response: &str) -> ExecutionPlan {
        let mut plan = ExecutionPlan::new(original_task);
        let lower = response.to_lowercase();
        let mut task_id = 0;

        // Look for coding tasks
        if lower.contains("implement")
            || lower.contains("create")
            || lower.contains("code")
            || lower.contains("build")
        {
            task_id += 1;
            plan = plan.with_task(PlanTask::coding(
                format!("task-{}", task_id),
                "Implement the feature as described",
            ));
        }

        // Add review task if code was generated
        let coding_ids: Vec<String> = plan.coding_tasks().iter().map(|t| t.id.clone()).collect();
        if !coding_ids.is_empty() && plan.requires_review {
            task_id += 1;
            plan = plan.with_task(
                PlanTask::review(format!("task-{}", task_id), "Review the generated code")
                    .with_dependencies(coding_ids.clone())
                    .with_priority(2),
            );
        }

        // Add test task if code was generated
        if !coding_ids.is_empty() && plan.requires_tests {
            task_id += 1;
            plan = plan.with_task(
                PlanTask::test(
                    format!("task-{}", task_id),
                    "Generate tests for the implementation",
                )
                .with_dependencies(coding_ids)
                .with_priority(3),
            );
        }

        plan
    }
}

impl Default for PlanParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_plan() {
        let parser = PlanParser::new();
        let json_response = r#"
        Here's the plan:
        {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "feature_description": "User authentication",
            "tasks": [
                {
                    "id": "auth-1",
                    "agent_type": "coder",
                    "description": "Create login form",
                    "depends_on": [],
                    "priority": 1
                }
            ],
            "requires_review": true,
            "requires_tests": true
        }
        "#;

        let plan = parser.parse("Auth feature", json_response);
        assert_eq!(plan.feature_description, "User authentication");
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, "auth-1");
    }

    #[test]
    fn test_parse_heuristic() {
        let parser = PlanParser::new();
        let text_response = "I will implement the user login feature by creating the necessary components.";

        let plan = parser.parse("Login feature", text_response);
        assert!(!plan.tasks.is_empty());
        assert!(!plan.coding_tasks().is_empty());
    }

    #[test]
    fn test_parse_empty_response() {
        let parser = PlanParser::new();
        let plan = parser.parse("Some feature", "The feature looks good.");

        // Should create empty plan when no implementation indicators
        assert!(plan.coding_tasks().is_empty());
    }
}
