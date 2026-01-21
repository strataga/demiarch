//! Agent context management for hierarchical execution
//!
//! The AgentContext tracks:
//! - Parent-child relationships in the agent hierarchy
//! - Shared resources (LLM client, cost tracker, etc.)
//! - Execution path for debugging and observability
//! - Message history and context for child agents

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::traits::{AgentResult, AgentStatus};
use super::AgentType;
use crate::cost::CostTracker;
use crate::llm::{LlmClient, Message};

/// Unique identifier for an agent instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new random agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Create from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Short form for display: first 8 chars
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Path through the agent hierarchy (e.g., "orchestrator/planner/coder-1")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentPath {
    /// Segments of the path
    segments: Vec<String>,
}

impl AgentPath {
    /// Create a new empty path (root)
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Create a path with a single segment
    pub fn new(segment: impl Into<String>) -> Self {
        Self {
            segments: vec![segment.into()],
        }
    }

    /// Create a child path by appending a segment
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Self { segments }
    }

    /// Get the depth of this path (0 for root)
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the parent path (None if root)
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            None
        } else {
            let mut segments = self.segments.clone();
            segments.pop();
            Some(Self { segments })
        }
    }

    /// Get the leaf segment (None if root)
    pub fn leaf(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Get all segments
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

impl std::fmt::Display for AgentPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_root() {
            write!(f, "/")
        } else {
            write!(f, "/{}", self.segments.join("/"))
        }
    }
}

/// Information about a child agent's execution
#[derive(Debug, Clone)]
pub struct ChildAgentInfo {
    /// Unique ID of the child agent
    pub id: AgentId,
    /// Type of the child agent
    pub agent_type: AgentType,
    /// Path in the hierarchy
    pub path: AgentPath,
    /// Current status
    pub status: AgentStatus,
    /// Result (if completed)
    pub result: Option<AgentResult>,
}

/// Shared state across the agent hierarchy
#[derive(Clone)]
pub struct SharedAgentState {
    /// LLM client for making API calls
    pub llm_client: Arc<LlmClient>,
    /// Cost tracker for budget enforcement
    pub cost_tracker: Option<Arc<CostTracker>>,
    /// Project ID this execution is for
    pub project_id: Option<Uuid>,
    /// Feature ID being worked on
    pub feature_id: Option<Uuid>,
    /// Global counter for generating unique names
    counter: Arc<AtomicU64>,
    /// Registry of all active agents
    agent_registry: Arc<RwLock<HashMap<AgentId, ChildAgentInfo>>>,
}

impl SharedAgentState {
    /// Create new shared state with required LLM client
    pub fn new(llm_client: Arc<LlmClient>) -> Self {
        Self {
            llm_client,
            cost_tracker: None,
            project_id: None,
            feature_id: None,
            counter: Arc::new(AtomicU64::new(0)),
            agent_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the cost tracker
    pub fn with_cost_tracker(mut self, tracker: Arc<CostTracker>) -> Self {
        self.cost_tracker = Some(tracker);
        self
    }

    /// Set the project ID
    pub fn with_project_id(mut self, id: Uuid) -> Self {
        self.project_id = Some(id);
        self
    }

    /// Set the feature ID
    pub fn with_feature_id(mut self, id: Uuid) -> Self {
        self.feature_id = Some(id);
        self
    }

    /// Get the next unique counter value
    pub fn next_counter(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Register an agent in the global registry
    pub async fn register_agent(&self, info: ChildAgentInfo) {
        let mut registry = self.agent_registry.write().await;
        registry.insert(info.id, info);
    }

    /// Update an agent's status in the registry
    pub async fn update_agent_status(&self, id: AgentId, status: AgentStatus) {
        let mut registry = self.agent_registry.write().await;
        if let Some(info) = registry.get_mut(&id) {
            info.status = status;
        }
    }

    /// Mark an agent as completed with its result
    pub async fn complete_agent(&self, id: AgentId, result: AgentResult) {
        let mut registry = self.agent_registry.write().await;
        if let Some(info) = registry.get_mut(&id) {
            info.status = if result.success {
                AgentStatus::Completed
            } else {
                AgentStatus::Failed
            };
            info.result = Some(result);
        }
    }

    /// Get info about a specific agent
    pub async fn get_agent(&self, id: AgentId) -> Option<ChildAgentInfo> {
        let registry = self.agent_registry.read().await;
        registry.get(&id).cloned()
    }

    /// Get all active agents
    pub async fn get_active_agents(&self) -> Vec<ChildAgentInfo> {
        let registry = self.agent_registry.read().await;
        registry
            .values()
            .filter(|info| info.status.is_active())
            .cloned()
            .collect()
    }

    /// Get all agents of a specific type
    pub async fn get_agents_by_type(&self, agent_type: AgentType) -> Vec<ChildAgentInfo> {
        let registry = self.agent_registry.read().await;
        registry
            .values()
            .filter(|info| info.agent_type == agent_type)
            .cloned()
            .collect()
    }
}

impl std::fmt::Debug for SharedAgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedAgentState")
            .field("project_id", &self.project_id)
            .field("feature_id", &self.feature_id)
            .field("has_cost_tracker", &self.cost_tracker.is_some())
            .finish()
    }
}

/// Context passed to agents during execution
///
/// Contains parent information, shared resources, and methods
/// for spawning child agents.
#[derive(Clone)]
pub struct AgentContext {
    /// Unique ID for this agent instance
    pub id: AgentId,
    /// Type of the current agent
    pub agent_type: AgentType,
    /// Path in the hierarchy
    pub path: AgentPath,
    /// Parent agent ID (None if this is the orchestrator)
    pub parent_id: Option<AgentId>,
    /// Depth in the hierarchy (0 for orchestrator)
    pub depth: u8,
    /// Message history inherited from parent
    pub inherited_messages: Vec<Message>,
    /// Shared state across the hierarchy
    shared_state: Arc<SharedAgentState>,
    /// Results from child agents
    child_results: Arc<RwLock<Vec<AgentResult>>>,
}

impl AgentContext {
    /// Create a root context for the orchestrator
    pub fn root(agent_type: AgentType, shared_state: Arc<SharedAgentState>) -> Self {
        let id = AgentId::new();
        Self {
            id,
            agent_type,
            path: AgentPath::new(agent_type.to_string()),
            parent_id: None,
            depth: 0,
            inherited_messages: Vec::new(),
            shared_state,
            child_results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a child context for spawning a new agent
    pub fn child_context(&self, child_type: AgentType) -> Self {
        let counter = self.shared_state.next_counter();
        let child_name = format!("{}-{}", child_type, counter);

        Self {
            id: AgentId::new(),
            agent_type: child_type,
            path: self.path.child(&child_name),
            parent_id: Some(self.id),
            depth: self.depth + 1,
            inherited_messages: self.inherited_messages.clone(),
            shared_state: Arc::clone(&self.shared_state),
            child_results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add messages to be inherited by child agents
    pub fn with_inherited_messages(mut self, messages: Vec<Message>) -> Self {
        self.inherited_messages = messages;
        self
    }

    /// Get the LLM client
    pub fn llm_client(&self) -> &Arc<LlmClient> {
        &self.shared_state.llm_client
    }

    /// Get the cost tracker (if configured)
    pub fn cost_tracker(&self) -> Option<&Arc<CostTracker>> {
        self.shared_state.cost_tracker.as_ref()
    }

    /// Get the project ID
    pub fn project_id(&self) -> Option<Uuid> {
        self.shared_state.project_id
    }

    /// Get the feature ID
    pub fn feature_id(&self) -> Option<Uuid> {
        self.shared_state.feature_id
    }

    /// Get the shared state
    pub fn shared_state(&self) -> &Arc<SharedAgentState> {
        &self.shared_state
    }

    /// Register this agent with the shared registry
    pub async fn register(&self) {
        let info = ChildAgentInfo {
            id: self.id,
            agent_type: self.agent_type,
            path: self.path.clone(),
            status: AgentStatus::Running,
            result: None,
        };
        self.shared_state.register_agent(info).await;
    }

    /// Update this agent's status
    pub async fn update_status(&self, status: AgentStatus) {
        self.shared_state.update_agent_status(self.id, status).await;
    }

    /// Mark this agent as completed
    pub async fn complete(&self, result: AgentResult) {
        self.shared_state.complete_agent(self.id, result).await;
    }

    /// Add a child agent's result
    pub async fn add_child_result(&self, result: AgentResult) {
        let mut results = self.child_results.write().await;
        results.push(result);
    }

    /// Get all child results
    pub async fn get_child_results(&self) -> Vec<AgentResult> {
        let results = self.child_results.read().await;
        results.clone()
    }

    /// Check if this agent can spawn a child of the given type
    pub fn can_spawn(&self, child_type: AgentType) -> bool {
        self.agent_type.can_spawn(child_type) && self.depth < 2
    }

    /// Get allowed child types for this agent
    pub fn allowed_children(&self) -> &'static [AgentType] {
        self.agent_type.allowed_children()
    }

    /// Check if we're at maximum depth
    pub fn at_max_depth(&self) -> bool {
        self.depth >= 2
    }
}

impl std::fmt::Debug for AgentContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentContext")
            .field("id", &self.id)
            .field("agent_type", &self.agent_type)
            .field("path", &self.path.to_string())
            .field("parent_id", &self.parent_id)
            .field("depth", &self.depth)
            .field("inherited_messages_count", &self.inherited_messages.len())
            .finish()
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
    fn test_agent_id() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);

        let display = id1.to_string();
        assert_eq!(display.len(), 8);
    }

    #[test]
    fn test_agent_path_root() {
        let path = AgentPath::root();
        assert!(path.is_root());
        assert_eq!(path.depth(), 0);
        assert!(path.parent().is_none());
        assert!(path.leaf().is_none());
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn test_agent_path_single() {
        let path = AgentPath::new("orchestrator");
        assert!(!path.is_root());
        assert_eq!(path.depth(), 1);
        assert_eq!(path.leaf(), Some("orchestrator"));
        assert_eq!(path.to_string(), "/orchestrator");
    }

    #[test]
    fn test_agent_path_nested() {
        let root = AgentPath::new("orchestrator");
        let child = root.child("planner-0");
        let grandchild = child.child("coder-1");

        assert_eq!(grandchild.depth(), 3);
        assert_eq!(grandchild.to_string(), "/orchestrator/planner-0/coder-1");

        let parent = grandchild.parent().unwrap();
        assert_eq!(parent.to_string(), "/orchestrator/planner-0");
    }

    #[tokio::test]
    async fn test_shared_agent_state() {
        let state = SharedAgentState::new(test_llm_client());
        let counter1 = state.next_counter();
        let counter2 = state.next_counter();

        assert_eq!(counter1, 0);
        assert_eq!(counter2, 1);
    }

    #[tokio::test]
    async fn test_agent_registry() {
        let state = SharedAgentState::new(test_llm_client());

        let info = ChildAgentInfo {
            id: AgentId::new(),
            agent_type: AgentType::Planner,
            path: AgentPath::new("planner-0"),
            status: AgentStatus::Running,
            result: None,
        };

        let agent_id = info.id;
        state.register_agent(info).await;

        let retrieved = state.get_agent(agent_id).await.unwrap();
        assert_eq!(retrieved.agent_type, AgentType::Planner);
        assert_eq!(retrieved.status, AgentStatus::Running);

        state.update_agent_status(agent_id, AgentStatus::Completed).await;
        let updated = state.get_agent(agent_id).await.unwrap();
        assert_eq!(updated.status, AgentStatus::Completed);
    }

    #[test]
    fn test_agent_context_root() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let ctx = AgentContext::root(AgentType::Orchestrator, shared);

        assert_eq!(ctx.depth, 0);
        assert!(ctx.parent_id.is_none());
        assert_eq!(ctx.agent_type, AgentType::Orchestrator);
    }

    #[test]
    fn test_agent_context_child() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let parent_ctx = AgentContext::root(AgentType::Orchestrator, shared);

        let child_ctx = parent_ctx.child_context(AgentType::Planner);

        assert_eq!(child_ctx.depth, 1);
        assert_eq!(child_ctx.parent_id, Some(parent_ctx.id));
        assert_eq!(child_ctx.agent_type, AgentType::Planner);
        assert!(child_ctx.path.to_string().contains("planner-"));
    }

    #[test]
    fn test_agent_context_can_spawn() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        // Orchestrator can spawn planner
        let orchestrator = AgentContext::root(AgentType::Orchestrator, shared.clone());
        assert!(orchestrator.can_spawn(AgentType::Planner));
        assert!(!orchestrator.can_spawn(AgentType::Coder));

        // Planner can spawn workers
        let planner = orchestrator.child_context(AgentType::Planner);
        assert!(planner.can_spawn(AgentType::Coder));
        assert!(planner.can_spawn(AgentType::Reviewer));
        assert!(!planner.can_spawn(AgentType::Planner));

        // Coder cannot spawn
        let coder = planner.child_context(AgentType::Coder);
        assert!(!coder.can_spawn(AgentType::Coder));
        assert!(coder.at_max_depth());
    }

    #[tokio::test]
    async fn test_agent_context_child_results() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let ctx = AgentContext::root(AgentType::Orchestrator, shared);

        let result1 = AgentResult::success("result 1");
        let result2 = AgentResult::success("result 2");

        ctx.add_child_result(result1).await;
        ctx.add_child_result(result2).await;

        let results = ctx.get_child_results().await;
        assert_eq!(results.len(), 2);
    }
}
