//! Agent trait and common types for hierarchical agent system

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use super::context::AgentContext;
use super::AgentType;
use crate::error::Result;
use crate::llm::Message;

/// Result of an agent's execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Whether the agent completed successfully
    pub success: bool,
    /// Output content from the agent
    pub output: String,
    /// Any artifacts produced (e.g., file paths, code blocks)
    pub artifacts: Vec<AgentArtifact>,
    /// Token usage for this agent's execution
    pub tokens_used: u32,
    /// Child agent results (for non-leaf agents)
    pub child_results: Vec<AgentResult>,
}

impl AgentResult {
    /// Create a successful result with output
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            artifacts: Vec::new(),
            tokens_used: 0,
            child_results: Vec::new(),
        }
    }

    /// Create a failed result with error message
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: error.into(),
            artifacts: Vec::new(),
            tokens_used: 0,
            child_results: Vec::new(),
        }
    }

    /// Add an artifact to the result
    pub fn with_artifact(mut self, artifact: AgentArtifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Add multiple artifacts to the result
    pub fn with_artifacts(mut self, artifacts: Vec<AgentArtifact>) -> Self {
        self.artifacts.extend(artifacts);
        self
    }

    /// Set token usage
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = tokens;
        self
    }

    /// Add a child result
    pub fn with_child_result(mut self, child: AgentResult) -> Self {
        self.child_results.push(child);
        self
    }

    /// Get total tokens used including children
    pub fn total_tokens(&self) -> u32 {
        self.tokens_used
            + self
                .child_results
                .iter()
                .map(|c| c.total_tokens())
                .sum::<u32>()
    }

    /// Check if all operations succeeded (including children)
    pub fn all_succeeded(&self) -> bool {
        self.success && self.child_results.iter().all(|c| c.all_succeeded())
    }
}

impl Default for AgentResult {
    fn default() -> Self {
        Self::success("")
    }
}

/// An artifact produced by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArtifact {
    /// Type of artifact
    pub artifact_type: ArtifactType,
    /// Name or identifier
    pub name: String,
    /// Content of the artifact
    pub content: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl AgentArtifact {
    /// Create a new artifact
    pub fn new(
        artifact_type: ArtifactType,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            artifact_type,
            name: name.into(),
            content: content.into(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a code artifact
    pub fn code(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(ArtifactType::Code, path, content)
    }

    /// Create a file artifact
    pub fn file(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(ArtifactType::File, path, content)
    }

    /// Create a review artifact
    pub fn review(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(ArtifactType::Review, name, content)
    }

    /// Create a test artifact
    pub fn test(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(ArtifactType::Test, name, content)
    }

    /// Create a plan artifact
    pub fn plan(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(ArtifactType::Plan, name, content)
    }

    /// Add metadata to the artifact
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Types of artifacts that agents can produce
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Generated code
    Code,
    /// File content
    File,
    /// Code review feedback
    Review,
    /// Test code
    Test,
    /// Execution plan
    Plan,
    /// Documentation
    Documentation,
    /// Other artifact type
    Other,
}

/// Status of an agent's execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is ready to execute
    Ready,
    /// Agent is currently running
    Running,
    /// Agent is waiting for child agent(s)
    WaitingForChildren,
    /// Agent completed successfully
    Completed,
    /// Agent failed with an error
    Failed,
    /// Agent was cancelled
    Cancelled,
}

impl AgentStatus {
    /// Check if the agent is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the agent is currently active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::WaitingForChildren)
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready => write!(f, "ready"),
            Self::Running => write!(f, "running"),
            Self::WaitingForChildren => write!(f, "waiting"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Capabilities that an agent can have
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    /// Can generate code
    CodeGeneration,
    /// Can review code
    CodeReview,
    /// Can generate tests
    TestGeneration,
    /// Can create execution plans
    Planning,
    /// Can coordinate other agents
    Orchestration,
    /// Can read files
    FileRead,
    /// Can write files
    FileWrite,
    /// Can execute commands
    CommandExecution,
    /// Can search the codebase
    CodebaseSearch,
}

/// Input for an agent task
#[derive(Debug, Clone)]
pub struct AgentInput {
    /// The task or request for the agent
    pub task: String,
    /// Additional context as messages
    pub context_messages: Vec<Message>,
    /// Parameters for the task
    pub parameters: serde_json::Value,
}

impl AgentInput {
    /// Create a new agent input
    pub fn new(task: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            context_messages: Vec::new(),
            parameters: serde_json::Value::Null,
        }
    }

    /// Add context messages
    pub fn with_context(mut self, messages: Vec<Message>) -> Self {
        self.context_messages = messages;
        self
    }

    /// Add parameters
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = params;
        self
    }
}

/// The core Agent trait for hierarchical agents
///
/// All agents implement this trait to participate in the Russian Doll hierarchy.
/// Agents receive input, optionally spawn child agents, and return results.
pub trait Agent: Send + Sync {
    /// Get the type of this agent
    fn agent_type(&self) -> AgentType;

    /// Get the capabilities this agent has
    fn capabilities(&self) -> &[AgentCapability];

    /// Get the current status of the agent
    fn status(&self) -> AgentStatus;

    /// Execute the agent's task
    ///
    /// This is the main entry point for agent execution. The agent receives
    /// input and context, performs its task (potentially spawning child agents),
    /// and returns a result.
    fn execute(
        &self,
        input: AgentInput,
        context: AgentContext,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResult>> + Send + '_>>;

    /// Get the system prompt for this agent
    fn system_prompt(&self) -> String;

    /// Get the maximum depth this agent can spawn children to
    /// Returns 0 for leaf agents
    fn max_child_depth(&self) -> u8 {
        if self.agent_type().is_leaf() {
            0
        } else {
            3 - self.agent_type().level()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_result_success() {
        let result = AgentResult::success("Task completed");
        assert!(result.success);
        assert_eq!(result.output, "Task completed");
        assert!(result.artifacts.is_empty());
    }

    #[test]
    fn test_agent_result_failure() {
        let result = AgentResult::failure("Something went wrong");
        assert!(!result.success);
        assert_eq!(result.output, "Something went wrong");
    }

    #[test]
    fn test_agent_result_with_artifacts() {
        let result = AgentResult::success("Done")
            .with_artifact(AgentArtifact::code("src/main.rs", "fn main() {}"))
            .with_artifact(AgentArtifact::test("test_main", "assert!(true)"));

        assert_eq!(result.artifacts.len(), 2);
        assert_eq!(result.artifacts[0].artifact_type, ArtifactType::Code);
        assert_eq!(result.artifacts[1].artifact_type, ArtifactType::Test);
    }

    #[test]
    fn test_agent_result_total_tokens() {
        let child1 = AgentResult::success("child1").with_tokens(100);
        let child2 = AgentResult::success("child2").with_tokens(200);

        let parent = AgentResult::success("parent")
            .with_tokens(50)
            .with_child_result(child1)
            .with_child_result(child2);

        assert_eq!(parent.total_tokens(), 350);
    }

    #[test]
    fn test_agent_result_all_succeeded() {
        let child1 = AgentResult::success("child1");
        let child2 = AgentResult::success("child2");
        let parent = AgentResult::success("parent")
            .with_child_result(child1)
            .with_child_result(child2);

        assert!(parent.all_succeeded());

        let failed_child = AgentResult::failure("oops");
        let parent_with_failure =
            AgentResult::success("parent").with_child_result(failed_child);
        assert!(!parent_with_failure.all_succeeded());
    }

    #[test]
    fn test_agent_status_is_terminal() {
        assert!(!AgentStatus::Ready.is_terminal());
        assert!(!AgentStatus::Running.is_terminal());
        assert!(!AgentStatus::WaitingForChildren.is_terminal());
        assert!(AgentStatus::Completed.is_terminal());
        assert!(AgentStatus::Failed.is_terminal());
        assert!(AgentStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_agent_status_is_active() {
        assert!(!AgentStatus::Ready.is_active());
        assert!(AgentStatus::Running.is_active());
        assert!(AgentStatus::WaitingForChildren.is_active());
        assert!(!AgentStatus::Completed.is_active());
    }

    #[test]
    fn test_agent_artifact_constructors() {
        let code = AgentArtifact::code("src/lib.rs", "pub fn hello() {}");
        assert_eq!(code.artifact_type, ArtifactType::Code);
        assert_eq!(code.name, "src/lib.rs");

        let review = AgentArtifact::review("review-1", "LGTM");
        assert_eq!(review.artifact_type, ArtifactType::Review);

        let test = AgentArtifact::test("test_hello", "#[test] fn test_hello() {}");
        assert_eq!(test.artifact_type, ArtifactType::Test);

        let plan = AgentArtifact::plan("implementation-plan", "1. Do X\n2. Do Y");
        assert_eq!(plan.artifact_type, ArtifactType::Plan);
    }

    #[test]
    fn test_agent_input() {
        let input = AgentInput::new("Generate a function")
            .with_parameters(serde_json::json!({"language": "rust"}));

        assert_eq!(input.task, "Generate a function");
        assert_eq!(input.parameters["language"], "rust");
    }
}
