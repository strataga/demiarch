//! Orchestrator agent - top-level session coordinator (Level 1)
//!
//! The Orchestrator is the entry point for all feature generation requests.
//! It coordinates the overall process and delegates to Planner agents.

use std::future::Future;
use std::pin::Pin;

use tracing::{debug, info};

use super::AgentType;
use super::context::AgentContext;
use super::message_builder::build_messages_from_input;
use super::planner::PlannerAgent;
use super::status::StatusTracker;
use super::traits::{Agent, AgentArtifact, AgentCapability, AgentInput, AgentResult, AgentStatus};
use crate::error::Result;
use crate::llm::Message;

/// Orchestrator agent - top-level coordinator for feature generation
///
/// The Orchestrator:
/// - Receives high-level feature requests from users
/// - Analyzes the request and prepares context
/// - Delegates to Planner agents for task decomposition
/// - Aggregates results from child agents
/// - Returns complete feature implementations
pub struct OrchestratorAgent {
    /// Current execution status
    status: StatusTracker,
    /// Available capabilities
    capabilities: Vec<AgentCapability>,
}

impl OrchestratorAgent {
    /// Create a new Orchestrator agent
    pub fn new() -> Self {
        Self {
            status: StatusTracker::new(),
            capabilities: vec![AgentCapability::Orchestration, AgentCapability::Planning],
        }
    }

    /// Execute the orchestration logic
    async fn orchestrate(&self, input: AgentInput, context: AgentContext) -> Result<AgentResult> {
        info!(
            agent_id = %context.id,
            path = %context.path,
            "Orchestrator starting task: {}",
            truncate_task(&input.task, 100)
        );

        // Register with the shared state (include task for monitoring)
        context.register_with_task(Some(&input.task)).await;

        // Update status to running
        self.status.set(AgentStatus::Running);
        context.update_status(AgentStatus::Running).await;

        // Build the system prompt and messages for the LLM
        let messages = self.build_messages(&input, &context);

        // Call the LLM to analyze the request and plan the approach
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
            "Orchestrator received LLM response"
        );

        // Parse the response to determine if we need to spawn child agents
        let plan = self.parse_orchestration_plan(&response.content);

        let result = if plan.needs_planner {
            // Spawn a Planner agent to decompose the feature
            self.status.set(AgentStatus::WaitingForChildren);
            context.update_status(AgentStatus::WaitingForChildren).await;

            let planner = PlannerAgent::new();
            let planner_context = context.child_context(AgentType::Planner);

            // Prepare input for the planner with refined task
            let planner_input = AgentInput::new(&plan.refined_task)
                .with_context(context.inherited_messages.clone());

            // Execute the planner
            let planner_result = planner.execute(planner_input, planner_context).await?;

            // Add the planner result as a child
            context.add_child_result(planner_result.clone()).await;

            // Build final result
            AgentResult::success(&response.content)
                .with_tokens(response.tokens_used)
                .with_artifact(AgentArtifact::plan(
                    "orchestration-plan",
                    &plan.refined_task,
                ))
                .with_child_result(planner_result)
        } else {
            // Simple request that doesn't need planning
            AgentResult::success(&response.content).with_tokens(response.tokens_used)
        };

        // Mark as completed
        self.status.set(AgentStatus::Completed);
        context.complete(result.clone()).await;

        info!(
            agent_id = %context.id,
            tokens = result.total_tokens(),
            success = result.success,
            children = result.child_results.len(),
            "Orchestrator completed"
        );

        Ok(result)
    }

    /// Build messages for the LLM call
    ///
    /// Extends the base message builder with project/feature context.
    fn build_messages(&self, input: &AgentInput, context: &AgentContext) -> Vec<Message> {
        // Start with the base messages
        let mut messages = build_messages_from_input(&self.system_prompt(), input, context);

        // Insert project context before the user message if available
        let insert_pos = messages.len().saturating_sub(1);

        if let Some(project_id) = context.project_id() {
            messages.insert(
                insert_pos,
                Message::system(format!("Working on project: {}", project_id)),
            );
        }

        if let Some(feature_id) = context.feature_id() {
            messages.insert(
                insert_pos,
                Message::system(format!("Implementing feature: {}", feature_id)),
            );
        }

        messages
    }

    /// Parse the LLM response to determine orchestration plan
    fn parse_orchestration_plan(&self, response: &str) -> OrchestrationPlan {
        // Simple heuristic: if the response mentions code generation,
        // implementation, or multiple steps, we need a planner
        let lower = response.to_lowercase();
        let needs_planner = lower.contains("implement")
            || lower.contains("generate")
            || lower.contains("create")
            || lower.contains("build")
            || lower.contains("step 1")
            || lower.contains("first,")
            || lower.contains("1.")
            || lower.contains("tasks:");

        OrchestrationPlan {
            needs_planner,
            refined_task: response.to_string(),
        }
    }

}

impl Default for OrchestratorAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for OrchestratorAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Orchestrator
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
        Box::pin(self.orchestrate(input, context))
    }

    fn system_prompt(&self) -> String {
        r#"You are the Orchestrator agent in a hierarchical code generation system.

Your role is to:
1. Analyze feature requests from users
2. Determine the scope and complexity of the request
3. Decide whether the request needs task decomposition
4. Provide a high-level plan for implementation

When analyzing a request:
- Identify the main components or modules needed
- Consider dependencies between components
- Estimate the complexity (simple, moderate, complex)
- Determine if code generation is required

For complex requests that need implementation:
- Break down into logical steps
- Identify what code needs to be written
- Note any tests that should be created
- Consider code review requirements

Your output will be used to guide the Planner agent, which will create
detailed tasks for the Coder, Reviewer, and Tester agents.

Be concise but thorough. Focus on actionable guidance."#
            .to_string()
    }
}

/// Internal plan structure from orchestration
struct OrchestrationPlan {
    /// Whether we need to spawn a Planner agent
    needs_planner: bool,
    /// The refined task description for the planner
    refined_task: String,
}

/// Truncate a task description for logging
fn truncate_task(task: &str, max_len: usize) -> String {
    if task.len() <= max_len {
        task.to_string()
    } else {
        format!("{}...", &task[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = OrchestratorAgent::new();
        assert_eq!(orchestrator.agent_type(), AgentType::Orchestrator);
        assert_eq!(orchestrator.status(), AgentStatus::Ready);
        assert!(
            orchestrator
                .capabilities()
                .contains(&AgentCapability::Orchestration)
        );
    }

    #[test]
    fn test_orchestrator_default() {
        let orchestrator = OrchestratorAgent::default();
        assert_eq!(orchestrator.agent_type(), AgentType::Orchestrator);
    }

    #[test]
    fn test_parse_orchestration_plan_needs_planner() {
        let orchestrator = OrchestratorAgent::new();

        // Should need planner
        let plan = orchestrator.parse_orchestration_plan(
            "I will implement the user authentication system. Step 1: Create the login form.",
        );
        assert!(plan.needs_planner);

        let plan =
            orchestrator.parse_orchestration_plan("Let me generate the code for this feature.");
        assert!(plan.needs_planner);

        let plan =
            orchestrator.parse_orchestration_plan("Tasks: 1. Create model 2. Add controller");
        assert!(plan.needs_planner);
    }

    #[test]
    fn test_parse_orchestration_plan_no_planner() {
        let orchestrator = OrchestratorAgent::new();

        // Should not need planner (simple queries)
        let plan = orchestrator.parse_orchestration_plan("The project is well structured.");
        assert!(!plan.needs_planner);

        let plan = orchestrator
            .parse_orchestration_plan("I understand your question about the architecture.");
        assert!(!plan.needs_planner);
    }

    #[test]
    fn test_system_prompt() {
        let orchestrator = OrchestratorAgent::new();
        let prompt = orchestrator.system_prompt();
        assert!(prompt.contains("Orchestrator"));
        assert!(prompt.contains("Planner"));
    }

    #[test]
    fn test_truncate_task() {
        assert_eq!(truncate_task("short", 10), "short");
        assert_eq!(truncate_task("this is a longer task", 10), "this is a ...");
    }

    #[test]
    fn test_max_child_depth() {
        let orchestrator = OrchestratorAgent::new();
        // Orchestrator is level 1, so max depth should be 2 (can have 2 levels below)
        assert_eq!(orchestrator.max_child_depth(), 2);
    }
}
