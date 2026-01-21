//! AgentTool - tool for spawning nested agents in the hierarchy
//!
//! The AgentTool allows parent agents to spawn child agents as part of
//! the Russian Doll pattern. It enforces hierarchy rules and manages
//! the delegation of tasks between agent levels.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::coder::CoderAgent;
use super::context::{AgentContext, SharedAgentState};
use super::orchestrator::OrchestratorAgent;
use super::planner::PlannerAgent;
use super::reviewer::ReviewerAgent;
use super::tester::TesterAgent;
use super::traits::{Agent, AgentInput, AgentResult};
use super::AgentType;
use crate::error::{Error, Result};
use crate::llm::LlmClient;

/// Result from an AgentTool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResult {
    /// The type of agent that was spawned
    pub agent_type: String,
    /// Whether the agent completed successfully
    pub success: bool,
    /// Output from the agent
    pub output: String,
    /// Token count from this agent and its children
    pub total_tokens: u32,
    /// Number of child agents spawned
    pub children_spawned: usize,
}

impl From<AgentResult> for AgentToolResult {
    fn from(result: AgentResult) -> Self {
        let total_tokens = result.total_tokens();
        let children_spawned = result.child_results.len();
        Self {
            agent_type: String::new(), // Will be set by caller
            success: result.success,
            output: result.output,
            total_tokens,
            children_spawned,
        }
    }
}

/// Tool for spawning nested agents
///
/// The AgentTool is the primary mechanism for implementing the Russian Doll
/// pattern. Parent agents use this tool to delegate work to child agents.
///
/// ## Hierarchy Rules
///
/// - Orchestrator (Level 1) can spawn Planner
/// - Planner (Level 2) can spawn Coder, Reviewer, Tester
/// - Coder, Reviewer, Tester (Level 3) cannot spawn children
pub struct AgentTool {
    /// Shared state for all agents in the hierarchy
    shared_state: Arc<SharedAgentState>,
}

impl AgentTool {
    /// Create a new AgentTool with the given LLM client
    pub fn new(llm_client: Arc<LlmClient>) -> Self {
        Self {
            shared_state: Arc::new(SharedAgentState::new(llm_client)),
        }
    }

    /// Create from existing shared state
    pub fn from_shared_state(shared_state: Arc<SharedAgentState>) -> Self {
        Self { shared_state }
    }

    /// Configure with project and feature IDs
    pub fn with_project(mut self, project_id: uuid::Uuid) -> Self {
        // We need to rebuild the shared state since it's immutable
        let llm_client = Arc::clone(&self.shared_state.llm_client);
        let mut new_state = SharedAgentState::new(llm_client).with_project_id(project_id);
        if let Some(tracker) = self.shared_state.cost_tracker.clone() {
            new_state = new_state.with_cost_tracker(tracker);
        }
        if let Some(feature_id) = self.shared_state.feature_id {
            new_state = new_state.with_feature_id(feature_id);
        }
        self.shared_state = Arc::new(new_state);
        self
    }

    /// Configure with feature ID
    pub fn with_feature(mut self, feature_id: uuid::Uuid) -> Self {
        let llm_client = Arc::clone(&self.shared_state.llm_client);
        let mut new_state = SharedAgentState::new(llm_client).with_feature_id(feature_id);
        if let Some(tracker) = self.shared_state.cost_tracker.clone() {
            new_state = new_state.with_cost_tracker(tracker);
        }
        if let Some(project_id) = self.shared_state.project_id {
            new_state = new_state.with_project_id(project_id);
        }
        self.shared_state = Arc::new(new_state);
        self
    }

    /// Start a new agent hierarchy by spawning an Orchestrator
    ///
    /// This is the entry point for feature generation requests.
    pub async fn spawn_orchestrator(&self, task: impl Into<String>) -> Result<AgentToolResult> {
        let task = task.into();
        info!(task = %truncate(&task, 50), "Spawning orchestrator for task");

        let orchestrator = OrchestratorAgent::new();
        let context = AgentContext::root(AgentType::Orchestrator, Arc::clone(&self.shared_state));
        let input = AgentInput::new(task);

        let result = orchestrator.execute(input, context).await?;

        Ok(AgentToolResult {
            agent_type: AgentType::Orchestrator.to_string(),
            ..AgentToolResult::from(result)
        })
    }

    /// Spawn a child agent from a parent context
    ///
    /// This validates that the spawn is allowed by the hierarchy rules.
    pub async fn spawn_child(
        &self,
        parent_context: &AgentContext,
        child_type: AgentType,
        task: impl Into<String>,
    ) -> Result<AgentToolResult> {
        let task = task.into();

        // Validate the spawn is allowed
        if !parent_context.can_spawn(child_type) {
            return Err(Error::InvalidInput(format!(
                "Agent type {} cannot spawn {} (current depth: {})",
                parent_context.agent_type, child_type, parent_context.depth
            )));
        }

        debug!(
            parent = %parent_context.agent_type,
            child = %child_type,
            task = %truncate(&task, 50),
            "Spawning child agent"
        );

        let child_context = parent_context.child_context(child_type);
        let input = AgentInput::new(task);

        let result = match child_type {
            AgentType::Planner => {
                let agent = PlannerAgent::new();
                agent.execute(input, child_context).await?
            }
            AgentType::Coder => {
                let agent = CoderAgent::new();
                agent.execute(input, child_context).await?
            }
            AgentType::Reviewer => {
                let agent = ReviewerAgent::new();
                agent.execute(input, child_context).await?
            }
            AgentType::Tester => {
                let agent = TesterAgent::new();
                agent.execute(input, child_context).await?
            }
            AgentType::Orchestrator => {
                return Err(Error::InvalidInput(
                    "Cannot spawn Orchestrator as a child agent".to_string(),
                ));
            }
        };

        Ok(AgentToolResult {
            agent_type: child_type.to_string(),
            ..AgentToolResult::from(result)
        })
    }

    /// Spawn multiple child agents in parallel
    ///
    /// All agents must be of types that the parent can spawn.
    pub async fn spawn_children_parallel(
        &self,
        parent_context: &AgentContext,
        tasks: Vec<(AgentType, String)>,
    ) -> Result<Vec<AgentToolResult>> {
        // Validate all spawns first
        for (child_type, _) in &tasks {
            if !parent_context.can_spawn(*child_type) {
                return Err(Error::InvalidInput(format!(
                    "Agent type {} cannot spawn {} (current depth: {})",
                    parent_context.agent_type, child_type, parent_context.depth
                )));
            }
        }

        // Spawn all agents in parallel
        let futures: Vec<_> = tasks
            .into_iter()
            .map(|(child_type, task)| self.spawn_child(parent_context, child_type, task))
            .collect();

        let results = futures_util::future::join_all(futures).await;

        // Collect results, propagating first error
        let mut agent_results = Vec::new();
        for result in results {
            agent_results.push(result?);
        }

        Ok(agent_results)
    }

    /// Get the shared state for advanced usage
    pub fn shared_state(&self) -> &Arc<SharedAgentState> {
        &self.shared_state
    }
}

impl std::fmt::Debug for AgentTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentTool")
            .field("shared_state", &self.shared_state)
            .finish()
    }
}

/// Truncate a string for logging
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LlmConfig;

    fn test_llm_client() -> Arc<LlmClient> {
        Arc::new(
            LlmClient::new(LlmConfig::default(), "test-key").expect("Failed to create LLM client"),
        )
    }

    #[test]
    fn test_agent_tool_creation() {
        let tool = AgentTool::new(test_llm_client());
        assert!(tool.shared_state().project_id.is_none());
    }

    #[test]
    fn test_agent_tool_with_project() {
        let tool = AgentTool::new(test_llm_client())
            .with_project(uuid::Uuid::new_v4());
        assert!(tool.shared_state().project_id.is_some());
    }

    #[test]
    fn test_agent_tool_with_feature() {
        let tool = AgentTool::new(test_llm_client())
            .with_feature(uuid::Uuid::new_v4());
        assert!(tool.shared_state().feature_id.is_some());
    }

    #[test]
    fn test_agent_tool_result_from_agent_result() {
        let agent_result = AgentResult::success("Test output")
            .with_tokens(100)
            .with_child_result(AgentResult::success("child").with_tokens(50));

        let tool_result: AgentToolResult = agent_result.into();
        assert!(tool_result.success);
        assert_eq!(tool_result.output, "Test output");
        assert_eq!(tool_result.total_tokens, 150);
        assert_eq!(tool_result.children_spawned, 1);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a longer string", 10), "this is a ...");
    }

    #[test]
    fn test_spawn_validation() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        // Orchestrator context
        let orchestrator_ctx = AgentContext::root(AgentType::Orchestrator, shared.clone());
        assert!(orchestrator_ctx.can_spawn(AgentType::Planner));
        assert!(!orchestrator_ctx.can_spawn(AgentType::Coder));

        // Planner context
        let planner_ctx = orchestrator_ctx.child_context(AgentType::Planner);
        assert!(planner_ctx.can_spawn(AgentType::Coder));
        assert!(planner_ctx.can_spawn(AgentType::Reviewer));
        assert!(planner_ctx.can_spawn(AgentType::Tester));
        assert!(!planner_ctx.can_spawn(AgentType::Planner));

        // Coder context (leaf - cannot spawn)
        let coder_ctx = planner_ctx.child_context(AgentType::Coder);
        assert!(!coder_ctx.can_spawn(AgentType::Coder));
        assert!(!coder_ctx.can_spawn(AgentType::Tester));
    }
}
