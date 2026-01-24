//! Tree visualization for agent hierarchy
//!
//! This module provides:
//! - AgentTreeNode: A tree representation of agents and their children
//! - TreeBuilder: Constructs trees from SharedAgentState registry
//! - HierarchyTree: Renders trees as formatted text for CLI/TUI display

use std::collections::HashMap;

use crate::agents::context::{ChildAgentInfo, SharedAgentState};
use crate::agents::events::{read_current_session_events, AgentEvent, AgentEventType};
use crate::agents::{AgentContext, AgentId, AgentPath, AgentStatus, AgentType};

/// Style configuration for tree rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeStyle {
    /// ASCII characters only (works everywhere)
    Ascii,
    /// Unicode box-drawing characters
    #[default]
    Unicode,
    /// Rounded Unicode style with status indicators
    Rounded,
}

/// Status icons for agent states
#[derive(Debug, Clone, Copy)]
pub struct StatusIcon;

impl StatusIcon {
    /// Get the icon for a given agent status
    pub fn for_status(status: AgentStatus, style: NodeStyle) -> &'static str {
        match style {
            NodeStyle::Ascii => match status {
                AgentStatus::Ready => "[.]",
                AgentStatus::Running => "[>]",
                AgentStatus::WaitingForChildren => "[~]",
                AgentStatus::Completed => "[+]",
                AgentStatus::Failed => "[X]",
                AgentStatus::Cancelled => "[-]",
            },
            NodeStyle::Unicode | NodeStyle::Rounded => match status {
                AgentStatus::Ready => "○",
                AgentStatus::Running => "●",
                AgentStatus::WaitingForChildren => "◐",
                AgentStatus::Completed => "✓",
                AgentStatus::Failed => "✗",
                AgentStatus::Cancelled => "⊘",
            },
        }
    }

    /// Get the icon for a given agent type
    pub fn for_agent_type(agent_type: AgentType, style: NodeStyle) -> &'static str {
        match style {
            NodeStyle::Ascii => match agent_type {
                AgentType::Orchestrator => "[O]",
                AgentType::Planner => "[P]",
                AgentType::Coder => "[C]",
                AgentType::Reviewer => "[R]",
                AgentType::Tester => "[T]",
            },
            NodeStyle::Unicode | NodeStyle::Rounded => match agent_type {
                AgentType::Orchestrator => "🎭",
                AgentType::Planner => "📋",
                AgentType::Coder => "💻",
                AgentType::Reviewer => "🔍",
                AgentType::Tester => "🧪",
            },
        }
    }
}

/// Options for rendering the tree
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Visual style for tree characters
    pub style: NodeStyle,
    /// Show agent IDs
    pub show_ids: bool,
    /// Show agent paths
    pub show_paths: bool,
    /// Show token usage
    pub show_tokens: bool,
    /// Show status icons
    pub show_status: bool,
    /// Show agent type icons
    pub show_type_icons: bool,
    /// Indentation width per level
    pub indent_width: usize,
    /// Maximum depth to render (-1 for unlimited)
    pub max_depth: i32,
    /// Colorize output (for terminals that support it)
    pub colorize: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            style: NodeStyle::Unicode,
            show_ids: true,
            show_paths: false,
            show_tokens: true,
            show_status: true,
            show_type_icons: true,
            indent_width: 4,
            max_depth: -1,
            colorize: true,
        }
    }
}

impl RenderOptions {
    /// Create ASCII-only options for basic terminals
    pub fn ascii() -> Self {
        Self {
            style: NodeStyle::Ascii,
            show_type_icons: false,
            ..Default::default()
        }
    }

    /// Create minimal options (no extras)
    pub fn minimal() -> Self {
        Self {
            show_ids: false,
            show_paths: false,
            show_tokens: false,
            show_status: true,
            show_type_icons: false,
            colorize: false,
            ..Default::default()
        }
    }

    /// Builder: set style
    pub fn with_style(mut self, style: NodeStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder: set max depth
    pub fn with_max_depth(mut self, depth: i32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Builder: enable/disable colors
    pub fn with_colors(mut self, colorize: bool) -> Self {
        self.colorize = colorize;
        self
    }
}

/// A node in the agent hierarchy tree
#[derive(Debug, Clone)]
pub struct AgentTreeNode {
    /// Unique agent identifier
    pub id: AgentId,
    /// Type of agent
    pub agent_type: AgentType,
    /// Current status
    pub status: AgentStatus,
    /// Path in the hierarchy
    pub path: AgentPath,
    /// Depth in tree (0 = root)
    pub depth: usize,
    /// Token usage (if available)
    pub tokens_used: Option<u32>,
    /// Total tokens including children (if available)
    pub total_tokens: Option<u32>,
    /// Success state (if completed)
    pub success: Option<bool>,
    /// Child nodes
    pub children: Vec<AgentTreeNode>,
}

impl AgentTreeNode {
    /// Create a new tree node from agent info
    pub fn from_agent_info(info: &ChildAgentInfo) -> Self {
        let (tokens_used, total_tokens, success) = if let Some(ref result) = info.result {
            (
                Some(result.tokens_used),
                Some(result.total_tokens()),
                Some(result.success),
            )
        } else {
            (None, None, None)
        };

        Self {
            id: info.id,
            agent_type: info.agent_type,
            status: info.status,
            path: info.path.clone(),
            depth: info.path.depth().saturating_sub(1), // Adjust since path includes agent name
            tokens_used,
            total_tokens,
            success,
            children: Vec::new(),
        }
    }

    /// Create a placeholder root node (when no orchestrator exists yet)
    pub fn placeholder_root() -> Self {
        Self {
            id: AgentId::new(),
            agent_type: AgentType::Orchestrator,
            status: AgentStatus::Ready,
            path: AgentPath::new("orchestrator"),
            depth: 0,
            tokens_used: None,
            total_tokens: None,
            success: None,
            children: Vec::new(),
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: AgentTreeNode) {
        self.children.push(child);
    }

    /// Find a node by ID (recursive)
    pub fn find_by_id(&self, id: AgentId) -> Option<&AgentTreeNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a mutable node by ID (recursive)
    pub fn find_by_id_mut(&mut self, id: AgentId) -> Option<&mut AgentTreeNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_by_id_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Count total nodes in tree
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }

    /// Count active (non-terminal) nodes
    pub fn count_active(&self) -> usize {
        let self_active = if self.status.is_active() { 1 } else { 0 };
        self_active
            + self
                .children
                .iter()
                .map(|c| c.count_active())
                .sum::<usize>()
    }

    /// Count completed nodes
    pub fn count_completed(&self) -> usize {
        let self_completed = if self.status == AgentStatus::Completed {
            1
        } else {
            0
        };
        self_completed
            + self
                .children
                .iter()
                .map(|c| c.count_completed())
                .sum::<usize>()
    }

    /// Count failed nodes
    pub fn count_failed(&self) -> usize {
        let self_failed = if self.status == AgentStatus::Failed {
            1
        } else {
            0
        };
        self_failed
            + self
                .children
                .iter()
                .map(|c| c.count_failed())
                .sum::<usize>()
    }

    /// Get total tokens used in tree
    pub fn tree_tokens(&self) -> u32 {
        self.tokens_used.unwrap_or(0) + self.children.iter().map(|c| c.tree_tokens()).sum::<u32>()
    }

    /// Check if all nodes succeeded
    pub fn all_succeeded(&self) -> bool {
        let self_ok = self.success.unwrap_or(true);
        self_ok && self.children.iter().all(|c| c.all_succeeded())
    }

    /// Get the display name for this node
    pub fn display_name(&self) -> String {
        self.path
            .leaf()
            .unwrap_or(&self.agent_type.to_string())
            .to_string()
    }
}

/// Builder for constructing agent trees from various sources
pub struct TreeBuilder;

impl TreeBuilder {
    /// Build a tree from the shared agent state registry
    pub async fn from_shared_state(state: &SharedAgentState) -> AgentTreeNode {
        // Get all agents from registry
        let registry = state.agent_registry_snapshot().await;
        Self::build_from_registry(&registry)
    }

    /// Build a tree from a registry snapshot
    pub fn build_from_registry(registry: &HashMap<AgentId, ChildAgentInfo>) -> AgentTreeNode {
        if registry.is_empty() {
            return AgentTreeNode::placeholder_root();
        }

        // Find root (orchestrator or smallest depth)
        let mut agents: Vec<_> = registry.values().collect();
        agents.sort_by_key(|a| a.path.depth());

        let root_info = agents.first().expect("registry not empty");
        let mut root = AgentTreeNode::from_agent_info(root_info);

        // Build parent-child map based on path relationships
        let mut path_to_node: HashMap<String, AgentTreeNode> = HashMap::new();
        path_to_node.insert(root.path.to_string(), root.clone());

        // Sort by depth to process parents before children
        for info in agents.iter().skip(1) {
            let node = AgentTreeNode::from_agent_info(info);
            path_to_node.insert(node.path.to_string(), node);
        }

        // Build tree structure by matching parent paths
        let mut root_children: Vec<AgentTreeNode> = Vec::new();

        for info in agents.iter().skip(1) {
            let node = AgentTreeNode::from_agent_info(info);
            if let Some(parent_path) = info.path.parent() {
                let parent_path_str = parent_path.to_string();
                if parent_path_str == root.path.to_string() {
                    root_children.push(node);
                }
            }
        }

        // Add level 3 children to level 2 nodes
        for child in &mut root_children {
            for info in agents.iter() {
                if let Some(parent_path) = info.path.parent() {
                    if parent_path.to_string() == child.path.to_string() {
                        let grandchild = AgentTreeNode::from_agent_info(info);
                        child.children.push(grandchild);
                    }
                }
            }
        }

        root.children = root_children;
        root
    }

    /// Build a tree from an AgentContext (follows parent chain)
    pub fn from_context(ctx: &AgentContext) -> AgentTreeNode {
        AgentTreeNode {
            id: ctx.id,
            agent_type: ctx.agent_type,
            status: AgentStatus::Running, // Context implies running
            path: ctx.path.clone(),
            depth: ctx.depth as usize,
            tokens_used: None,
            total_tokens: None,
            success: None,
            children: Vec::new(),
        }
    }

    /// Build a tree from live agent events (reads from JSONL file)
    ///
    /// This method reads the current session's events and reconstructs
    /// the agent tree from spawned/completed/failed events.
    pub fn from_live_events() -> AgentTreeNode {
        let events = read_current_session_events();
        Self::build_from_events(&events)
    }

    /// Build a tree from a list of agent events
    pub fn build_from_events(events: &[AgentEvent]) -> AgentTreeNode {
        if events.is_empty() {
            return AgentTreeNode::placeholder_root();
        }

        // Build a map of agent data from events
        let mut agents: HashMap<String, EventAgentInfo> = HashMap::new();

        for event in events {
            let id = &event.agent.id;

            match event.event_type {
                AgentEventType::Spawned => {
                    agents.insert(
                        id.clone(),
                        EventAgentInfo {
                            id: id.clone(),
                            agent_type: event.agent.agent_type.clone(),
                            name: event.agent.name.clone(),
                            parent_id: event.agent.parent_id.clone(),
                            path: event.agent.path.clone(),
                            status: AgentStatus::Running,
                            tokens: event.agent.tokens,
                            task: event.agent.task.clone(),
                        },
                    );
                }
                AgentEventType::StatusUpdate | AgentEventType::TokenUpdate => {
                    if let Some(info) = agents.get_mut(id) {
                        info.status = parse_status(&event.agent.status);
                        if event.agent.tokens > 0 {
                            info.tokens = event.agent.tokens;
                        }
                    }
                }
                AgentEventType::Completed => {
                    if let Some(info) = agents.get_mut(id) {
                        info.status = AgentStatus::Completed;
                        if event.agent.tokens > 0 {
                            info.tokens = event.agent.tokens;
                        }
                    }
                }
                AgentEventType::Failed => {
                    if let Some(info) = agents.get_mut(id) {
                        info.status = AgentStatus::Failed;
                    }
                }
                AgentEventType::Cancelled => {
                    if let Some(info) = agents.get_mut(id) {
                        info.status = AgentStatus::Cancelled;
                    }
                }
                AgentEventType::Started => {
                    if let Some(info) = agents.get_mut(id) {
                        info.status = AgentStatus::Running;
                    }
                }
            }
        }

        // Find root (orchestrator)
        let root_id = agents
            .values()
            .find(|a| a.parent_id.is_none())
            .map(|a| a.id.clone());

        if let Some(root_id) = root_id {
            build_tree_from_events_map(&agents, &root_id)
        } else {
            AgentTreeNode::placeholder_root()
        }
    }

    /// Build a demo/example tree for testing
    pub fn demo_tree() -> AgentTreeNode {
        let mut orchestrator = AgentTreeNode {
            id: AgentId::new(),
            agent_type: AgentType::Orchestrator,
            status: AgentStatus::WaitingForChildren,
            path: AgentPath::new("orchestrator"),
            depth: 0,
            tokens_used: Some(150),
            total_tokens: None,
            success: None,
            children: Vec::new(),
        };

        let mut planner = AgentTreeNode {
            id: AgentId::new(),
            agent_type: AgentType::Planner,
            status: AgentStatus::WaitingForChildren,
            path: orchestrator.path.child("planner-0"),
            depth: 1,
            tokens_used: Some(320),
            total_tokens: None,
            success: None,
            children: Vec::new(),
        };

        let coder = AgentTreeNode {
            id: AgentId::new(),
            agent_type: AgentType::Coder,
            status: AgentStatus::Running,
            path: planner.path.child("coder-0"),
            depth: 2,
            tokens_used: Some(580),
            total_tokens: Some(580),
            success: None,
            children: Vec::new(),
        };

        let reviewer = AgentTreeNode {
            id: AgentId::new(),
            agent_type: AgentType::Reviewer,
            status: AgentStatus::Completed,
            path: planner.path.child("reviewer-1"),
            depth: 2,
            tokens_used: Some(210),
            total_tokens: Some(210),
            success: Some(true),
            children: Vec::new(),
        };

        let tester = AgentTreeNode {
            id: AgentId::new(),
            agent_type: AgentType::Tester,
            status: AgentStatus::Ready,
            path: planner.path.child("tester-2"),
            depth: 2,
            tokens_used: None,
            total_tokens: None,
            success: None,
            children: Vec::new(),
        };

        planner.children = vec![coder, reviewer, tester];
        orchestrator.children = vec![planner];
        orchestrator.total_tokens = Some(orchestrator.tree_tokens());

        orchestrator
    }
}

/// Main tree visualization renderer
pub struct HierarchyTree {
    root: AgentTreeNode,
    options: RenderOptions,
}

impl HierarchyTree {
    /// Create a new hierarchy tree renderer
    pub fn new(root: AgentTreeNode) -> Self {
        Self {
            root,
            options: RenderOptions::default(),
        }
    }

    /// Create with custom render options
    pub fn with_options(root: AgentTreeNode, options: RenderOptions) -> Self {
        Self { root, options }
    }

    /// Get the root node
    pub fn root(&self) -> &AgentTreeNode {
        &self.root
    }

    /// Get mutable root node
    pub fn root_mut(&mut self) -> &mut AgentTreeNode {
        &mut self.root
    }

    /// Get render options
    pub fn options(&self) -> &RenderOptions {
        &self.options
    }

    /// Set render options
    pub fn set_options(&mut self, options: RenderOptions) {
        self.options = options;
    }

    /// Render the tree to a string
    pub fn render(&self) -> String {
        let mut output = String::new();
        self.render_node(&self.root, &mut output, "", true, 0);
        output
    }

    /// Render to a vector of lines (useful for TUI)
    pub fn render_lines(&self) -> Vec<String> {
        self.render().lines().map(|s| s.to_string()).collect()
    }

    /// Render with a summary header
    pub fn render_with_summary(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&self.render_header());
        output.push('\n');

        // Tree
        self.render_node(&self.root, &mut output, "", true, 0);

        // Footer stats
        output.push('\n');
        output.push_str(&self.render_footer());

        output
    }

    fn render_header(&self) -> String {
        let total = self.root.count();
        let active = self.root.count_active();
        let completed = self.root.count_completed();
        let failed = self.root.count_failed();

        format!(
            "Agent Hierarchy ({} total, {} active, {} completed, {} failed)",
            total, active, completed, failed
        )
    }

    fn render_footer(&self) -> String {
        let tokens = self.root.tree_tokens();
        format!("Total tokens: {}", tokens)
    }

    fn render_node(
        &self,
        node: &AgentTreeNode,
        output: &mut String,
        prefix: &str,
        is_last: bool,
        depth: usize,
    ) {
        // Check max depth
        if self.options.max_depth >= 0 && depth as i32 > self.options.max_depth {
            return;
        }

        // Get tree characters based on style
        let (branch, vertical) = match self.options.style {
            NodeStyle::Ascii => {
                if is_last {
                    ("`-- ", "    ")
                } else {
                    ("+-- ", "|   ")
                }
            }
            NodeStyle::Unicode => {
                if is_last {
                    ("└── ", "    ")
                } else {
                    ("├── ", "│   ")
                }
            }
            NodeStyle::Rounded => {
                if is_last {
                    ("╰── ", "    ")
                } else {
                    ("├── ", "│   ")
                }
            }
        };

        // Build the node line
        let mut line = String::new();

        // Prefix and branch
        if depth > 0 {
            line.push_str(prefix);
            line.push_str(branch);
        }

        // Type icon
        if self.options.show_type_icons {
            line.push_str(StatusIcon::for_agent_type(
                node.agent_type,
                self.options.style,
            ));
            line.push(' ');
        }

        // Status icon
        if self.options.show_status {
            line.push_str(StatusIcon::for_status(node.status, self.options.style));
            line.push(' ');
        }

        // Agent name/type
        line.push_str(&node.display_name());

        // ID
        if self.options.show_ids {
            line.push_str(&format!(" [{}]", node.id));
        }

        // Path
        if self.options.show_paths {
            line.push_str(&format!(" ({})", node.path));
        }

        // Tokens
        if self.options.show_tokens {
            if let Some(tokens) = node.tokens_used {
                line.push_str(&format!(" {}tok", tokens));
            }
        }

        // Status text for terminal states
        if node.status.is_terminal() {
            line.push_str(&format!(" [{}]", node.status));
        }

        output.push_str(&line);
        output.push('\n');

        // Render children
        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            let new_prefix = if depth > 0 {
                format!("{}{}", prefix, vertical)
            } else {
                String::new()
            };
            self.render_node(child, output, &new_prefix, is_last_child, depth + 1);
        }
    }

    /// Render a compact single-line status
    pub fn render_compact(&self) -> String {
        let total = self.root.count();
        let active = self.root.count_active();
        let completed = self.root.count_completed();
        let failed = self.root.count_failed();
        let tokens = self.root.tree_tokens();

        format!(
            "[{}/{} agents] Active:{} Done:{} Failed:{} Tokens:{}",
            active, total, active, completed, failed, tokens
        )
    }
}

impl std::fmt::Display for HierarchyTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Helper struct for building trees from events
#[derive(Debug, Clone)]
struct EventAgentInfo {
    id: String,
    agent_type: String,
    name: String,
    parent_id: Option<String>,
    path: String,
    status: AgentStatus,
    tokens: u64,
    #[allow(dead_code)]
    task: Option<String>,
}

/// Parse status string to AgentStatus enum
fn parse_status(s: &str) -> AgentStatus {
    match s.to_lowercase().as_str() {
        "ready" | "spawned" => AgentStatus::Ready,
        "running" => AgentStatus::Running,
        "waiting" | "waiting_for_children" => AgentStatus::WaitingForChildren,
        "completed" => AgentStatus::Completed,
        "failed" => AgentStatus::Failed,
        "cancelled" => AgentStatus::Cancelled,
        _ => AgentStatus::Running,
    }
}

/// Parse agent type string to AgentType enum
fn parse_agent_type(s: &str) -> AgentType {
    match s.to_lowercase().as_str() {
        "orchestrator" => AgentType::Orchestrator,
        "planner" => AgentType::Planner,
        "coder" => AgentType::Coder,
        "reviewer" => AgentType::Reviewer,
        "tester" => AgentType::Tester,
        _ => AgentType::Orchestrator,
    }
}

/// Build tree recursively from events map
fn build_tree_from_events_map(agents: &HashMap<String, EventAgentInfo>, id: &str) -> AgentTreeNode {
    let info = match agents.get(id) {
        Some(i) => i,
        None => return AgentTreeNode::placeholder_root(),
    };

    // Parse path into AgentPath
    let path = if info.path.is_empty() || info.path == "/" {
        AgentPath::new(&info.name)
    } else {
        // Path is like "/orchestrator/planner-0", need to build AgentPath
        let segments: Vec<&str> = info.path.trim_start_matches('/').split('/').collect();
        let mut path = AgentPath::root();
        for seg in segments {
            if !seg.is_empty() {
                path = path.child(seg);
            }
        }
        path
    };

    let depth = path.depth().saturating_sub(1);

    // Parse the agent ID from the string, or create a deterministic one from the string
    // Use a simple hash-based approach to create a stable UUID from the string ID
    let agent_id = AgentId::parse(&info.id).unwrap_or_else(|| {
        // Create a deterministic UUID from the string ID using a hash
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        info.id.hash(&mut hasher);
        let hash = hasher.finish();
        // Use the hash to create a stable UUID (version 4 format but deterministic)
        let bytes = [
            (hash >> 56) as u8,
            (hash >> 48) as u8,
            (hash >> 40) as u8,
            (hash >> 32) as u8,
            (hash >> 24) as u8,
            (hash >> 16) as u8,
            (hash >> 8) as u8,
            hash as u8,
            (hash >> 56) as u8,
            (hash >> 48) as u8,
            (hash >> 40) as u8,
            (hash >> 32) as u8,
            (hash >> 24) as u8,
            (hash >> 16) as u8,
            (hash >> 8) as u8,
            hash as u8,
        ];
        AgentId::from_uuid(uuid::Uuid::from_bytes(bytes))
    });

    let mut node = AgentTreeNode {
        id: agent_id,
        agent_type: parse_agent_type(&info.agent_type),
        status: info.status,
        path,
        depth,
        tokens_used: if info.tokens > 0 {
            Some(info.tokens as u32)
        } else {
            None
        },
        total_tokens: None,
        success: match info.status {
            AgentStatus::Completed => Some(true),
            AgentStatus::Failed => Some(false),
            _ => None,
        },
        children: Vec::new(),
    };

    // Find and add children
    for child_info in agents.values() {
        if child_info.parent_id.as_deref() == Some(id) {
            let child_node = build_tree_from_events_map(agents, &child_info.id);
            node.children.push(child_node);
        }
    }

    // Calculate total tokens
    node.total_tokens = Some(node.tree_tokens());

    node
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_tree_structure() {
        let tree = TreeBuilder::demo_tree();

        assert_eq!(tree.agent_type, AgentType::Orchestrator);
        assert_eq!(tree.children.len(), 1);

        let planner = &tree.children[0];
        assert_eq!(planner.agent_type, AgentType::Planner);
        assert_eq!(planner.children.len(), 3);

        assert_eq!(planner.children[0].agent_type, AgentType::Coder);
        assert_eq!(planner.children[1].agent_type, AgentType::Reviewer);
        assert_eq!(planner.children[2].agent_type, AgentType::Tester);
    }

    #[test]
    fn test_tree_counts() {
        let tree = TreeBuilder::demo_tree();

        assert_eq!(tree.count(), 5); // 1 orchestrator + 1 planner + 3 workers
        assert_eq!(tree.count_active(), 3); // orchestrator (waiting) + planner (waiting) + coder (running)
        assert_eq!(tree.count_completed(), 1); // reviewer
        assert_eq!(tree.count_failed(), 0);
    }

    #[test]
    fn test_tree_tokens() {
        let tree = TreeBuilder::demo_tree();
        // 150 (orchestrator) + 320 (planner) + 580 (coder) + 210 (reviewer) + 0 (tester)
        assert_eq!(tree.tree_tokens(), 1260);
    }

    #[test]
    fn test_render_unicode() {
        let tree = TreeBuilder::demo_tree();
        let renderer = HierarchyTree::new(tree);
        let output = renderer.render();

        // Should contain unicode tree characters
        assert!(output.contains("└──") || output.contains("├──"));
        // Should contain agent names
        assert!(output.contains("orchestrator"));
        assert!(output.contains("planner"));
        assert!(output.contains("coder"));
    }

    #[test]
    fn test_render_ascii() {
        let tree = TreeBuilder::demo_tree();
        let renderer = HierarchyTree::with_options(tree, RenderOptions::ascii());
        let output = renderer.render();

        // Should contain ASCII tree characters
        assert!(output.contains("`--") || output.contains("+--"));
        // Should NOT contain unicode
        assert!(!output.contains("└──"));
    }

    #[test]
    fn test_render_with_summary() {
        let tree = TreeBuilder::demo_tree();
        let renderer = HierarchyTree::new(tree);
        let output = renderer.render_with_summary();

        // Should have header with counts
        assert!(output.contains("Agent Hierarchy"));
        assert!(output.contains("5 total"));

        // Should have footer with tokens
        assert!(output.contains("Total tokens:"));
    }

    #[test]
    fn test_render_compact() {
        let tree = TreeBuilder::demo_tree();
        let renderer = HierarchyTree::new(tree);
        let compact = renderer.render_compact();

        assert!(compact.contains("agents"));
        assert!(compact.contains("Active:"));
        assert!(compact.contains("Tokens:"));
    }

    #[test]
    fn test_find_by_id() {
        let tree = TreeBuilder::demo_tree();
        let planner_id = tree.children[0].id;
        let coder_id = tree.children[0].children[0].id;

        assert!(tree.find_by_id(tree.id).is_some());
        assert!(tree.find_by_id(planner_id).is_some());
        assert!(tree.find_by_id(coder_id).is_some());
        assert!(tree.find_by_id(AgentId::new()).is_none());
    }

    #[test]
    fn test_status_icons() {
        assert_eq!(
            StatusIcon::for_status(AgentStatus::Running, NodeStyle::Unicode),
            "●"
        );
        assert_eq!(
            StatusIcon::for_status(AgentStatus::Completed, NodeStyle::Unicode),
            "✓"
        );
        assert_eq!(
            StatusIcon::for_status(AgentStatus::Failed, NodeStyle::Unicode),
            "✗"
        );

        assert_eq!(
            StatusIcon::for_status(AgentStatus::Running, NodeStyle::Ascii),
            "[>]"
        );
    }

    #[test]
    fn test_agent_type_icons() {
        assert_eq!(
            StatusIcon::for_agent_type(AgentType::Orchestrator, NodeStyle::Unicode),
            "🎭"
        );
        assert_eq!(
            StatusIcon::for_agent_type(AgentType::Coder, NodeStyle::Unicode),
            "💻"
        );
        assert_eq!(
            StatusIcon::for_agent_type(AgentType::Orchestrator, NodeStyle::Ascii),
            "[O]"
        );
    }

    #[test]
    fn test_render_options_builder() {
        let opts = RenderOptions::default()
            .with_style(NodeStyle::Ascii)
            .with_max_depth(2)
            .with_colors(false);

        assert_eq!(opts.style, NodeStyle::Ascii);
        assert_eq!(opts.max_depth, 2);
        assert!(!opts.colorize);
    }

    #[test]
    fn test_max_depth_rendering() {
        let tree = TreeBuilder::demo_tree();
        let opts = RenderOptions::default().with_max_depth(1);
        let renderer = HierarchyTree::with_options(tree, opts);
        let output = renderer.render();

        // Should show orchestrator and planner, but not coder/reviewer/tester
        assert!(output.contains("orchestrator"));
        assert!(output.contains("planner"));
        // Level 3 agents should not appear (max_depth=1 means depth 0 and 1)
        // Actually max_depth means we stop at that depth, so depth 0 and 1 are shown
        // Level 3 workers are at depth 2, which exceeds max_depth=1
    }

    #[test]
    fn test_placeholder_root() {
        let root = AgentTreeNode::placeholder_root();
        assert_eq!(root.agent_type, AgentType::Orchestrator);
        assert_eq!(root.status, AgentStatus::Ready);
        assert!(root.children.is_empty());
    }

    #[test]
    fn test_all_succeeded() {
        let mut tree = TreeBuilder::demo_tree();

        // Initially, nodes without explicit success flag default to true
        // Demo tree has reviewer with success=Some(true), others are None
        // None defaults to true, so all_succeeded should be true
        assert!(tree.all_succeeded());

        // Set one node to failure
        tree.children[0].children[0].success = Some(false);
        assert!(!tree.all_succeeded());

        // Fix the failure
        tree.children[0].children[0].success = Some(true);
        assert!(tree.all_succeeded());

        // Verify deep failure detection
        tree.children[0].children[1].success = Some(false);
        assert!(!tree.all_succeeded());
    }
}
