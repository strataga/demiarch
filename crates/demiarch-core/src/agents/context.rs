//! Agent context management for hierarchical execution
//!
//! The AgentContext tracks:
//! - Parent-child relationships in the agent hierarchy
//! - Shared resources (LLM client, cost tracker, etc.)
//! - Execution path for debugging and observability
//! - Message history and context for child agents
//! - Progressive disclosure settings for token management

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::events::AgentEventWriter;
use super::traits::{AgentResult, AgentStatus};
use super::AgentType;
use crate::context::{
    estimate_messages_tokens, ContextBudget, ContextWindow, DisclosureLevel, TokenAllocation,
};
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

    /// Parse from a string (full UUID or short 8-char form)
    /// Returns None if the string is not a valid UUID
    pub fn parse(s: &str) -> Option<Self> {
        // Try full UUID first
        if let Ok(uuid) = Uuid::parse_str(s) {
            return Some(Self(uuid));
        }
        // For short form (8 chars), we can't recover the full UUID
        // So return None - the caller should use the original string
        None
    }

    /// Get the full UUID string representation
    pub fn full_string(&self) -> String {
        self.0.to_string()
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
    /// Human-readable name (e.g., "planner-0", "coder-1")
    pub name: String,
    /// Parent agent ID (None for root orchestrator)
    pub parent_id: Option<AgentId>,
    /// Path in the hierarchy
    pub path: AgentPath,
    /// Current status
    pub status: AgentStatus,
    /// Task description this agent is working on
    pub task: Option<String>,
    /// Token usage accumulated by this agent
    pub tokens_used: u64,
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
    /// Filesystem path to the project directory (for writing generated files)
    pub project_path: Option<PathBuf>,
    /// Global counter for generating unique names
    counter: Arc<AtomicU64>,
    /// Registry of all active agents
    agent_registry: Arc<RwLock<HashMap<AgentId, ChildAgentInfo>>>,
    /// Context budget for token allocation
    context_budget: Arc<ContextBudget>,
    /// Event writer for real-time monitoring
    event_writer: Arc<AgentEventWriter>,
    /// Token for cancelling agent execution
    cancellation_token: CancellationToken,
}

impl SharedAgentState {
    /// Create new shared state with required LLM client
    pub fn new(llm_client: Arc<LlmClient>) -> Self {
        Self {
            llm_client,
            cost_tracker: None,
            project_id: None,
            feature_id: None,
            project_path: None,
            counter: Arc::new(AtomicU64::new(0)),
            agent_registry: Arc::new(RwLock::new(HashMap::new())),
            context_budget: Arc::new(ContextBudget::default()),
            event_writer: Arc::new(AgentEventWriter::new()),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Get the cancellation token
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// Cancel all agents in this hierarchy
    pub fn cancel_all(&self) {
        self.cancellation_token.cancel();
    }

    /// Create new shared state with custom context budget
    pub fn with_context_budget(llm_client: Arc<LlmClient>, budget: ContextBudget) -> Self {
        Self {
            llm_client,
            cost_tracker: None,
            project_id: None,
            feature_id: None,
            project_path: None,
            counter: Arc::new(AtomicU64::new(0)),
            agent_registry: Arc::new(RwLock::new(HashMap::new())),
            context_budget: Arc::new(budget),
            event_writer: Arc::new(AgentEventWriter::new()),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Get the event writer for emitting agent events
    pub fn event_writer(&self) -> &AgentEventWriter {
        &self.event_writer
    }

    /// Get the context budget
    pub fn context_budget(&self) -> &ContextBudget {
        &self.context_budget
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

    /// Set the project filesystem path
    pub fn with_project_path(mut self, path: PathBuf) -> Self {
        self.project_path = Some(path);
        self
    }

    /// Get the next unique counter value
    pub fn next_counter(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Register an agent in the global registry
    pub async fn register_agent(&self, info: ChildAgentInfo) {
        // Emit spawned event
        self.event_writer.emit_spawned(
            &info.id,
            info.agent_type,
            &info.name,
            info.parent_id.as_ref(),
            &info.path.to_string(),
            info.task.as_deref(),
        );

        let id = info.id;
        let mut registry = self.agent_registry.write().await;
        registry.insert(id, info);
    }

    /// Update an agent's status in the registry
    pub async fn update_agent_status(&self, id: AgentId, status: AgentStatus) {
        let tokens = {
            let registry = self.agent_registry.read().await;
            registry.get(&id).map(|i| i.tokens_used).unwrap_or(0)
        };

        // Emit status update event
        self.event_writer.emit_status_update(&id, status, tokens);

        let mut registry = self.agent_registry.write().await;
        if let Some(info) = registry.get_mut(&id) {
            info.status = status;
        }
    }

    /// Mark an agent as completed with its result
    pub async fn complete_agent(&self, id: AgentId, result: AgentResult) {
        let tokens = result.tokens_used as u64;

        // Emit completion event
        if result.success {
            self.event_writer.emit_completed(&id, tokens);
        } else {
            // For failures, the error message is in the output field
            self.event_writer.emit_failed(&id, &result.output);
        }

        let mut registry = self.agent_registry.write().await;
        if let Some(info) = registry.get_mut(&id) {
            info.status = if result.success {
                AgentStatus::Completed
            } else {
                AgentStatus::Failed
            };
            info.tokens_used = tokens;
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

    /// Get a snapshot of the agent registry for visualization
    ///
    /// Returns a clone of the current registry state, useful for
    /// building tree visualizations without holding locks.
    pub async fn agent_registry_snapshot(&self) -> HashMap<AgentId, ChildAgentInfo> {
        let registry = self.agent_registry.read().await;
        registry.clone()
    }

    /// Get all agents in the registry
    pub async fn get_all_agents(&self) -> Vec<ChildAgentInfo> {
        let registry = self.agent_registry.read().await;
        registry.values().cloned().collect()
    }
}

impl std::fmt::Debug for SharedAgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedAgentState")
            .field("project_id", &self.project_id)
            .field("feature_id", &self.feature_id)
            .field("project_path", &self.project_path)
            .field("has_cost_tracker", &self.cost_tracker.is_some())
            .field("context_budget_tokens", &self.context_budget.total_tokens)
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

    /// Get the project filesystem path
    pub fn project_path(&self) -> Option<&PathBuf> {
        self.shared_state.project_path.as_ref()
    }

    /// Get the shared state
    pub fn shared_state(&self) -> &Arc<SharedAgentState> {
        &self.shared_state
    }

    /// Get the cancellation token
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.shared_state.cancellation_token
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.shared_state.cancellation_token.is_cancelled()
    }

    /// Register this agent with the shared registry
    pub async fn register(&self) {
        self.register_with_task(None).await;
    }

    /// Register this agent with a task description
    pub async fn register_with_task(&self, task: Option<&str>) {
        let info = ChildAgentInfo {
            id: self.id,
            agent_type: self.agent_type,
            name: self.path.leaf().unwrap_or("root").to_string(),
            parent_id: self.parent_id,
            path: self.path.clone(),
            status: AgentStatus::Running,
            task: task.map(|t| t.to_string()),
            tokens_used: 0,
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

    // Progressive Disclosure Methods

    /// Get the disclosure level for this agent based on its depth
    pub fn disclosure_level(&self) -> DisclosureLevel {
        DisclosureLevel::for_depth(self.depth)
    }

    /// Get the token allocation for this agent based on its depth
    pub fn token_allocation(&self) -> TokenAllocation {
        self.shared_state
            .context_budget
            .allocation_for_depth(self.depth)
    }

    /// Create a context window for this agent
    ///
    /// Returns a ContextWindow configured with the appropriate token allocation
    /// and disclosure level for this agent's depth in the hierarchy.
    pub fn create_context_window(&self) -> ContextWindow {
        let allocation = self.token_allocation();
        ContextWindow::new(allocation).with_disclosure_level(self.disclosure_level())
    }

    /// Estimate total tokens used by inherited messages
    pub fn estimate_inherited_tokens(&self) -> usize {
        estimate_messages_tokens(&self.inherited_messages)
    }

    /// Get the remaining context budget for inherited messages
    ///
    /// Returns the number of tokens available for context after accounting
    /// for the token allocation strategy at this depth.
    pub fn remaining_context_budget(&self) -> usize {
        let allocation = self.token_allocation();
        allocation
            .context_tokens
            .saturating_sub(self.estimate_inherited_tokens())
    }

    /// Check if inherited messages exceed the context budget
    pub fn is_context_over_budget(&self) -> bool {
        let allocation = self.token_allocation();
        self.estimate_inherited_tokens() > allocation.context_tokens
    }

    /// Create a child context with progressive disclosure applied
    ///
    /// This method creates a child context where the inherited messages
    /// are compressed according to the child's disclosure level to manage
    /// token limits as context flows down the hierarchy.
    pub fn child_context_with_disclosure(&self, child_type: AgentType) -> Self {
        let counter = self.shared_state.next_counter();
        let child_name = format!("{}-{}", child_type, counter);
        let child_depth = self.depth + 1;

        // Get the disclosure level and allocation for the child
        let child_disclosure = DisclosureLevel::for_depth(child_depth);
        let child_allocation = self
            .shared_state
            .context_budget
            .allocation_for_depth(child_depth);

        // Create a context window for the parent to compress inherited messages
        let mut parent_window = self.create_context_window();
        for msg in &self.inherited_messages {
            parent_window.add_context_message(msg.clone());
        }

        // Create child window with compressed context
        let child_window = parent_window.child_window(child_allocation, child_depth);
        let compressed_messages = child_window.messages();

        // Log compression if significant
        let original_tokens = self.estimate_inherited_tokens();
        let compressed_tokens = estimate_messages_tokens(&compressed_messages);
        if original_tokens > 0 && compressed_tokens < original_tokens {
            tracing::debug!(
                parent_depth = self.depth,
                child_depth,
                disclosure = %child_disclosure,
                original_tokens,
                compressed_tokens,
                compression_ratio = %((compressed_tokens as f32 / original_tokens as f32 * 100.0).round()),
                "Progressive disclosure applied to child context"
            );
        }

        Self {
            id: AgentId::new(),
            agent_type: child_type,
            path: self.path.child(&child_name),
            parent_id: Some(self.id),
            depth: child_depth,
            inherited_messages: compressed_messages,
            shared_state: Arc::clone(&self.shared_state),
            child_results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get a summary of the current context state for debugging
    pub fn context_summary(&self) -> ContextSummary {
        let allocation = self.token_allocation();
        let inherited_tokens = self.estimate_inherited_tokens();

        ContextSummary {
            agent_id: self.id,
            agent_type: self.agent_type,
            depth: self.depth,
            disclosure_level: self.disclosure_level(),
            inherited_message_count: self.inherited_messages.len(),
            inherited_tokens,
            context_budget: allocation.context_tokens,
            budget_utilization: if allocation.context_tokens > 0 {
                inherited_tokens as f32 / allocation.context_tokens as f32
            } else {
                0.0
            },
        }
    }
}

/// Summary of context state for an agent
#[derive(Debug, Clone)]
pub struct ContextSummary {
    /// Agent ID
    pub agent_id: AgentId,
    /// Agent type
    pub agent_type: AgentType,
    /// Depth in hierarchy
    pub depth: u8,
    /// Disclosure level at this depth
    pub disclosure_level: DisclosureLevel,
    /// Number of inherited messages
    pub inherited_message_count: usize,
    /// Estimated tokens in inherited messages
    pub inherited_tokens: usize,
    /// Token budget for context
    pub context_budget: usize,
    /// Budget utilization (0.0 to 1.0+)
    pub budget_utilization: f32,
}

impl std::fmt::Display for ContextSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Agent {} ({}): depth={}, disclosure={}, messages={}, tokens={}/{} ({:.1}%)",
            self.agent_id,
            self.agent_type,
            self.depth,
            self.disclosure_level,
            self.inherited_message_count,
            self.inherited_tokens,
            self.context_budget,
            self.budget_utilization * 100.0
        )
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
            name: "planner-0".to_string(),
            parent_id: None,
            path: AgentPath::new("planner-0"),
            status: AgentStatus::Running,
            task: Some("Test task".to_string()),
            tokens_used: 0,
            result: None,
        };

        let agent_id = info.id;
        state.register_agent(info).await;

        let retrieved = state.get_agent(agent_id).await.unwrap();
        assert_eq!(retrieved.agent_type, AgentType::Planner);
        assert_eq!(retrieved.status, AgentStatus::Running);

        state
            .update_agent_status(agent_id, AgentStatus::Completed)
            .await;
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

    // Progressive Disclosure Tests

    #[test]
    fn test_disclosure_level_by_depth() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        let orchestrator = AgentContext::root(AgentType::Orchestrator, shared.clone());
        assert_eq!(orchestrator.disclosure_level(), DisclosureLevel::Full);

        let planner = orchestrator.child_context(AgentType::Planner);
        assert_eq!(planner.disclosure_level(), DisclosureLevel::Summary);

        let coder = planner.child_context(AgentType::Coder);
        assert_eq!(coder.disclosure_level(), DisclosureLevel::Essential);
    }

    #[test]
    fn test_token_allocation_by_depth() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        let orchestrator = AgentContext::root(AgentType::Orchestrator, shared.clone());
        let planner = orchestrator.child_context(AgentType::Planner);
        let coder = planner.child_context(AgentType::Coder);

        // Orchestrator should have more context budget than planner
        assert!(
            orchestrator.token_allocation().context_tokens
                > planner.token_allocation().context_tokens
        );
        // Planner should have more context budget than coder
        assert!(
            planner.token_allocation().context_tokens > coder.token_allocation().context_tokens
        );
    }

    #[test]
    fn test_create_context_window() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let ctx = AgentContext::root(AgentType::Orchestrator, shared);

        let window = ctx.create_context_window();
        assert_eq!(window.disclosure_level(), DisclosureLevel::Full);
    }

    #[test]
    fn test_estimate_inherited_tokens() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let ctx =
            AgentContext::root(AgentType::Orchestrator, shared).with_inherited_messages(vec![
                Message::user("Hello, this is a test message."),
                Message::assistant("This is a response."),
            ]);

        let tokens = ctx.estimate_inherited_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_remaining_context_budget() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let ctx = AgentContext::root(AgentType::Orchestrator, shared.clone());

        let budget = ctx.remaining_context_budget();
        assert!(budget > 0);

        // Add messages and check budget decreases
        let ctx_with_messages = AgentContext::root(AgentType::Orchestrator, shared)
            .with_inherited_messages(vec![Message::user("Hello, this is a test message.")]);

        assert!(ctx_with_messages.remaining_context_budget() < budget);
    }

    #[test]
    fn test_is_context_over_budget() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        let ctx = AgentContext::root(AgentType::Orchestrator, shared.clone());
        assert!(!ctx.is_context_over_budget());

        // Create a context with very long messages to exceed budget
        let long_message = "A".repeat(50000); // Very long message
        let ctx_over_budget = AgentContext::root(AgentType::Orchestrator, shared)
            .with_inherited_messages(vec![Message::user(&long_message)]);

        assert!(ctx_over_budget.is_context_over_budget());
    }

    #[test]
    fn test_child_context_with_disclosure() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        let orchestrator = AgentContext::root(AgentType::Orchestrator, shared)
            .with_inherited_messages(vec![
                Message::user("First message with important context."),
                Message::assistant("Response with details. Key point: use progressive disclosure."),
                Message::user("Another message with more context."),
            ]);

        let planner = orchestrator.child_context_with_disclosure(AgentType::Planner);

        // Child should have correct depth and disclosure level
        assert_eq!(planner.depth, 1);
        assert_eq!(planner.disclosure_level(), DisclosureLevel::Summary);

        // Child should have parent reference
        assert_eq!(planner.parent_id, Some(orchestrator.id));
    }

    #[test]
    fn test_context_summary() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));

        let ctx = AgentContext::root(AgentType::Orchestrator, shared)
            .with_inherited_messages(vec![Message::user("Test message.")]);

        let summary = ctx.context_summary();

        assert_eq!(summary.agent_type, AgentType::Orchestrator);
        assert_eq!(summary.depth, 0);
        assert_eq!(summary.disclosure_level, DisclosureLevel::Full);
        assert_eq!(summary.inherited_message_count, 1);
        assert!(summary.inherited_tokens > 0);
        assert!(summary.context_budget > 0);
    }

    #[test]
    fn test_context_summary_display() {
        let shared = Arc::new(SharedAgentState::new(test_llm_client()));
        let ctx = AgentContext::root(AgentType::Orchestrator, shared);
        let summary = ctx.context_summary();

        let display = summary.to_string();
        assert!(display.contains("orchestrator")); // AgentType displays as lowercase
        assert!(display.contains("depth=0"));
        assert!(display.contains("full"));
    }

    #[test]
    fn test_shared_state_with_context_budget() {
        let budget = ContextBudget::new(16384); // 16k context
        let state = SharedAgentState::with_context_budget(test_llm_client(), budget);

        assert_eq!(state.context_budget().total_tokens, 16384);
    }
}
