//! Planner agent - task decomposition and coordination (Level 2)
//!
//! The Planner decomposes features into actionable tasks and
//! coordinates Coder, Reviewer, and Tester agents.

use std::future::Future;
use std::pin::Pin;

use tracing::{debug, info};

use super::AgentType;
use super::coder::CoderAgent;
use super::context::AgentContext;
use super::message_builder::build_messages_from_input;
use super::reviewer::ReviewerAgent;
use super::status::StatusTracker;
use super::tester::TesterAgent;
use super::traits::{Agent, AgentArtifact, AgentCapability, AgentInput, AgentResult, AgentStatus, ArtifactType};
use crate::domain::feature_decomposition::{ExecutionPlan, PlanTask, TaskStatus};
use crate::error::Result;
use crate::llm::Message;

/// Planner agent - coordinates task decomposition
///
/// The Planner:
/// - Analyzes high-level feature descriptions
/// - Breaks features into discrete tasks
/// - Determines task dependencies and ordering
/// - Spawns Coder, Reviewer, and Tester agents
pub struct PlannerAgent {
    /// Current execution status
    status: StatusTracker,
    /// Available capabilities
    capabilities: Vec<AgentCapability>,
}

impl PlannerAgent {
    /// Create a new Planner agent
    pub fn new() -> Self {
        Self {
            status: StatusTracker::new(),
            capabilities: vec![AgentCapability::Planning, AgentCapability::CodebaseSearch],
        }
    }

    /// Execute the planning logic
    async fn plan(&self, input: AgentInput, context: AgentContext) -> Result<AgentResult> {
        // Check for cancellation at start
        if context.is_cancelled() {
            self.status.set(AgentStatus::Cancelled);
            return Ok(AgentResult::failure("Cancelled"));
        }

        info!(
            agent_id = %context.id,
            path = %context.path,
            "Planner starting task decomposition"
        );

        // Register with the shared state (include task for monitoring)
        context.register_with_task(Some(&input.task)).await;

        // Update status to running
        self.status.set(AgentStatus::Running);
        context.update_status(AgentStatus::Running).await;

        // Build messages for the LLM
        let messages = build_messages_from_input(&self.system_prompt(), &input, &context);

        // Call the LLM to create an execution plan
        let llm_client = context.llm_client();
        let response = match llm_client.complete(messages, None).await {
            Ok(resp) => resp,
            Err(e) => {
                self.status.set(AgentStatus::Failed);
                let result = AgentResult::failure(format!("LLM call failed: {}", e));
                context.complete(result.clone()).await;
                return Ok(result);
            }
        };

        debug!(
            tokens = response.tokens_used,
            "Planner received LLM response"
        );

        // Parse the execution plan from the response
        let plan = self.parse_execution_plan(&input.task, &response.content);

        // Create the plan artifact
        let plan_json = serde_json::to_string_pretty(&plan).unwrap_or_default();
        let mut result = AgentResult::success(&response.content)
            .with_tokens(response.tokens_used)
            .with_artifact(AgentArtifact::plan("execution-plan", &plan_json));

        // Execute the plan by spawning child agents
        if !plan.tasks.is_empty() {
            self.status.set(AgentStatus::WaitingForChildren);
            context.update_status(AgentStatus::WaitingForChildren).await;

            // Collect code artifacts from coder results to pass to reviewers/testers
            let mut code_artifacts: Vec<AgentArtifact> = Vec::new();

            // Execute coding tasks first
            for task in plan.coding_tasks() {
                // Check for cancellation before each task
                if context.is_cancelled() {
                    self.status.set(AgentStatus::Cancelled);
                    let cancelled_result = AgentResult::failure("Cancelled by user");
                    context.complete(cancelled_result.clone()).await;
                    return Ok(cancelled_result);
                }

                let coder_result = self.execute_coder_task(task, &context).await?;

                // Collect code artifacts for downstream agents
                code_artifacts.extend(
                    coder_result
                        .artifacts
                        .iter()
                        .filter(|a| a.artifact_type == ArtifactType::Code)
                        .cloned(),
                );

                context.add_child_result(coder_result.clone()).await;
                result = result.with_child_result(coder_result);
            }

            // Build code context for reviewers and testers
            let code_context = if !code_artifacts.is_empty() {
                let code_summary: String = code_artifacts
                    .iter()
                    .map(|a| format!("**{}**\n```\n{}\n```", a.name, a.content))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Some(code_summary)
            } else {
                None
            };

            // Then review if required
            if plan.requires_review {
                for task in plan.review_tasks() {
                    if context.is_cancelled() {
                        self.status.set(AgentStatus::Cancelled);
                        let cancelled_result = AgentResult::failure("Cancelled by user");
                        context.complete(cancelled_result.clone()).await;
                        return Ok(cancelled_result);
                    }

                    let review_result =
                        self.execute_reviewer_task(task, &context, code_context.as_deref())
                            .await?;
                    context.add_child_result(review_result.clone()).await;
                    result = result.with_child_result(review_result);
                }
            }

            // Finally tests if required
            if plan.requires_tests {
                for task in plan.test_tasks() {
                    if context.is_cancelled() {
                        self.status.set(AgentStatus::Cancelled);
                        let cancelled_result = AgentResult::failure("Cancelled by user");
                        context.complete(cancelled_result.clone()).await;
                        return Ok(cancelled_result);
                    }

                    let test_result =
                        self.execute_tester_task(task, &context, code_context.as_deref())
                            .await?;
                    context.add_child_result(test_result.clone()).await;
                    result = result.with_child_result(test_result);
                }
            }
        }

        // Mark as completed
        self.status.set(AgentStatus::Completed);
        context.complete(result.clone()).await;

        info!(
            agent_id = %context.id,
            tokens = result.total_tokens(),
            tasks = plan.tasks.len(),
            children = result.child_results.len(),
            "Planner completed"
        );

        Ok(result)
    }

    /// Execute a coding task with a Coder agent
    async fn execute_coder_task(
        &self,
        task: &PlanTask,
        context: &AgentContext,
    ) -> Result<AgentResult> {
        let coder = CoderAgent::new();
        let coder_context = context.child_context(AgentType::Coder);
        let coder_input = AgentInput::new(&task.description);

        coder.execute(coder_input, coder_context).await
    }

    /// Execute a review task with a Reviewer agent
    async fn execute_reviewer_task(
        &self,
        task: &PlanTask,
        context: &AgentContext,
        code_context: Option<&str>,
    ) -> Result<AgentResult> {
        let reviewer = ReviewerAgent::new();
        let reviewer_context = context.child_context(AgentType::Reviewer);

        // Include code context if available
        let task_with_context = if let Some(code) = code_context {
            format!(
                "{}\n\n## Code to Review\n\n{}",
                task.description, code
            )
        } else {
            task.description.clone()
        };

        let reviewer_input = AgentInput::new(task_with_context).with_context(
            if code_context.is_some() {
                vec![Message::user(
                    "Please review the code above for issues, best practices, and suggestions.",
                )]
            } else {
                vec![]
            },
        );

        reviewer.execute(reviewer_input, reviewer_context).await
    }

    /// Execute a test task with a Tester agent
    async fn execute_tester_task(
        &self,
        task: &PlanTask,
        context: &AgentContext,
        code_context: Option<&str>,
    ) -> Result<AgentResult> {
        let tester = TesterAgent::new();
        let tester_context = context.child_context(AgentType::Tester);

        // Include code context if available
        let task_with_context = if let Some(code) = code_context {
            format!(
                "{}\n\n## Code to Test\n\n{}",
                task.description, code
            )
        } else {
            task.description.clone()
        };

        let tester_input = AgentInput::new(task_with_context).with_context(
            if code_context.is_some() {
                vec![Message::user(
                    "Please write tests for the code above.",
                )]
            } else {
                vec![]
            },
        );

        tester.execute(tester_input, tester_context).await
    }

    /// Parse the LLM response into an execution plan
    fn parse_execution_plan(&self, original_task: &str, response: &str) -> ExecutionPlan {
        let mut plan = ExecutionPlan::new(original_task);

        // Try to parse JSON if present
        if let Some(json_start) = response.find('{')
            && let Some(json_end) = response.rfind('}')
        {
            let json_str = &response[json_start..=json_end];
            if let Ok(parsed) = serde_json::from_str::<ExecutionPlan>(json_str) {
                return parsed;
            }
        }

        // Fall back to heuristic parsing
        let lower = response.to_lowercase();
        let mut task_id = 0;

        // Look for coding tasks
        if lower.contains("implement") || lower.contains("create") || lower.contains("code") {
            task_id += 1;
            plan.tasks.push(PlanTask {
                id: format!("task-{}", task_id),
                agent_type: "coder".to_string(),
                description: "Implement the feature as described".to_string(),
                depends_on: vec![],
                priority: 1,
                status: TaskStatus::Pending,
            });
        }

        // Add review task if code was generated
        if !plan.coding_tasks().is_empty() && plan.requires_review {
            task_id += 1;
            let depends_on: Vec<String> =
                plan.coding_tasks().iter().map(|t| t.id.clone()).collect();
            plan.tasks.push(PlanTask {
                id: format!("task-{}", task_id),
                agent_type: "reviewer".to_string(),
                description: "Review the generated code".to_string(),
                depends_on,
                priority: 2,
                status: TaskStatus::Pending,
            });
        }

        // Add test task if code was generated
        if !plan.coding_tasks().is_empty() && plan.requires_tests {
            task_id += 1;
            let depends_on: Vec<String> =
                plan.coding_tasks().iter().map(|t| t.id.clone()).collect();
            plan.tasks.push(PlanTask {
                id: format!("task-{}", task_id),
                agent_type: "tester".to_string(),
                description: "Generate tests for the implementation".to_string(),
                depends_on,
                priority: 3,
                status: TaskStatus::Pending,
            });
        }

        plan
    }
}

impl Default for PlannerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for PlannerAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Planner
    }

    fn capabilities(&self) -> &[AgentCapability] {
        &self.capabilities
    }

    fn status(&self) -> AgentStatus {
        self.status.get()
    }

    fn execute(
        &self,
        input: AgentInput,
        context: AgentContext,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResult>> + Send + '_>> {
        Box::pin(self.plan(input, context))
    }

    fn system_prompt(&self) -> String {
        r#"You are the Planner agent in a hierarchical code generation system.

Your role is to:
1. Analyze feature descriptions from the Orchestrator
2. Break features into discrete, actionable tasks
3. Identify dependencies between tasks
4. Assign tasks to appropriate worker agents (Coder, Reviewer, Tester)

Task Types:
- **Coder**: Generate new code, modify existing code, create files
- **Reviewer**: Review generated code for quality, bugs, and best practices
- **Tester**: Generate unit tests, integration tests, ensure test coverage

Guidelines:
- Each task should be atomic and clearly defined
- Specify dependencies to ensure correct execution order
- Prioritize tasks (1 = highest priority)
- Include both the "what" and "why" for each task

Output your plan as JSON when possible:
```json
{
  "feature_description": "Brief description of the feature",
  "tasks": [
    {
      "id": "task-1",
      "agent_type": "coder",
      "description": "What needs to be done",
      "depends_on": [],
      "priority": 1
    }
  ],
  "requires_review": true,
  "requires_tests": true
}
```

Be thorough but efficient. Don't over-decompose simple features."#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_creation() {
        let planner = PlannerAgent::new();
        assert_eq!(planner.agent_type(), AgentType::Planner);
        assert_eq!(planner.status(), AgentStatus::Ready);
        assert!(planner.capabilities().contains(&AgentCapability::Planning));
    }

    #[test]
    fn test_planner_default() {
        let planner = PlannerAgent::default();
        assert_eq!(planner.agent_type(), AgentType::Planner);
    }

    #[test]
    fn test_execution_plan_creation() {
        let plan = ExecutionPlan::new("Test feature")
            .with_task(PlanTask {
                id: "task-1".to_string(),
                agent_type: "coder".to_string(),
                description: "Write code".to_string(),
                depends_on: vec![],
                priority: 1,
                status: TaskStatus::Pending,
            })
            .with_task(PlanTask {
                id: "task-2".to_string(),
                agent_type: "reviewer".to_string(),
                description: "Review code".to_string(),
                depends_on: vec!["task-1".to_string()],
                priority: 2,
                status: TaskStatus::Pending,
            });

        assert_eq!(plan.tasks.len(), 2);
        assert_eq!(plan.coding_tasks().len(), 1);
        assert_eq!(plan.review_tasks().len(), 1);
        assert_eq!(plan.test_tasks().len(), 0);
    }

    #[test]
    fn test_parse_execution_plan_json() {
        let planner = PlannerAgent::new();
        // Note: The domain ExecutionPlan requires an `id` field (UUID), so if missing
        // the JSON parse will fail and fallback to heuristic parsing
        let json_response = r#"
        Here's the plan:
        {
            "id": "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
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

        let plan = planner.parse_execution_plan("Auth feature", json_response);
        assert_eq!(plan.feature_description, "User authentication");
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, "auth-1");
    }

    #[test]
    fn test_parse_execution_plan_heuristic() {
        let planner = PlannerAgent::new();
        let text_response =
            "I will implement the user login feature by creating the necessary components.";

        let plan = planner.parse_execution_plan("Login feature", text_response);
        assert!(!plan.tasks.is_empty());
        assert!(!plan.coding_tasks().is_empty());
    }

    #[test]
    fn test_system_prompt() {
        let planner = PlannerAgent::new();
        let prompt = planner.system_prompt();
        assert!(prompt.contains("Planner"));
        assert!(prompt.contains("Coder"));
        assert!(prompt.contains("Reviewer"));
        assert!(prompt.contains("Tester"));
    }

    #[test]
    fn test_max_child_depth() {
        let planner = PlannerAgent::new();
        // Planner is level 2, so max depth should be 1 (only worker agents below)
        assert_eq!(planner.max_child_depth(), 1);
    }
}
