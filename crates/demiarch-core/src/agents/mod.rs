//! Agent system - Russian Doll hierarchical agents
//!
//! This module implements the Hierarchical Task Decomposition Agent Pattern:
//! "Agents within agents within agents."
//!
//! ## Three-Level Hierarchy
//!
//! | Level | Agent Type | Role | Delegates To |
//! |-------|------------|------|--------------|
//! | Level 1: Director | Orchestrator | Session coordinator | Planner |
//! | Level 2: Coordinator | Planner | Decomposes features into tasks | Coder, Reviewer, Tester |
//! | Level 3: Tool Agents | Coder, Reviewer, Tester | Execute specific tasks | (leaf nodes) |
//!
//! ## Execution Flow
//!
//! 1. User requests feature generation
//! 2. **Orchestrator** receives request, spawns **Planner** via `AgentTool` call
//! 3. **Planner** decomposes feature into tasks, spawns **Coder** agents
//! 4. **Coder** generates code, **Reviewer** validates, **Tester** creates tests
//! 5. Results bubble back up through the hierarchy
//! 6. **Orchestrator** returns complete feature implementation

pub mod code_extraction;
pub mod coder;
pub mod context;
pub mod events;
pub mod message_builder;
pub mod orchestrator;
pub mod planner;
pub mod reviewer;
pub mod status;
pub mod tester;
pub mod tool;
pub mod traits;

pub use coder::CoderAgent;
pub use context::{AgentContext, AgentId, AgentPath};
pub use events::{
    AgentEvent, AgentEventData, AgentEventReader, AgentEventType, AgentEventWriter, clear_events,
    read_current_session_events, read_recent_events,
};
pub use orchestrator::OrchestratorAgent;
pub use planner::PlannerAgent;
pub use reviewer::ReviewerAgent;
pub use tester::TesterAgent;
pub use tool::{AgentTool, AgentToolResult};
pub use traits::{Agent, AgentCapability, AgentResult, AgentStatus};
pub use message_builder::{
    build_agent_messages, build_enriched_agent_messages, build_enriched_messages_from_input,
    build_messages_from_input, build_messages_with_enrichment, EnrichedMessageBuilder,
    EnrichedMessageConfig,
};

use serde::{Deserialize, Serialize};

/// Agent types in the hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Top-level session coordinator (Level 1)
    Orchestrator,
    /// Feature decomposition and task planning (Level 2)
    Planner,
    /// Code generation (Level 3)
    Coder,
    /// Code review and validation (Level 3)
    Reviewer,
    /// Test generation (Level 3)
    Tester,
}

impl AgentType {
    /// Get the hierarchy level for this agent type (1-3)
    pub fn level(&self) -> u8 {
        match self {
            Self::Orchestrator => 1,
            Self::Planner => 2,
            Self::Coder | Self::Reviewer | Self::Tester => 3,
        }
    }

    /// Check if this agent type can spawn agents of the given type
    pub fn can_spawn(&self, child_type: AgentType) -> bool {
        match self {
            Self::Orchestrator => child_type == AgentType::Planner,
            Self::Planner => matches!(
                child_type,
                AgentType::Coder | AgentType::Reviewer | AgentType::Tester
            ),
            // Level 3 agents are leaf nodes and cannot spawn children
            Self::Coder | Self::Reviewer | Self::Tester => false,
        }
    }

    /// Get allowed child agent types for this agent
    pub fn allowed_children(&self) -> &'static [AgentType] {
        match self {
            Self::Orchestrator => &[AgentType::Planner],
            Self::Planner => &[AgentType::Coder, AgentType::Reviewer, AgentType::Tester],
            Self::Coder | Self::Reviewer | Self::Tester => &[],
        }
    }

    /// Check if this is a leaf agent (cannot spawn children)
    pub fn is_leaf(&self) -> bool {
        self.level() == 3
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Orchestrator => write!(f, "orchestrator"),
            Self::Planner => write!(f, "planner"),
            Self::Coder => write!(f, "coder"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::Tester => write!(f, "tester"),
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "orchestrator" => Ok(Self::Orchestrator),
            "planner" => Ok(Self::Planner),
            "coder" => Ok(Self::Coder),
            "reviewer" => Ok(Self::Reviewer),
            "tester" => Ok(Self::Tester),
            _ => Err(format!("Unknown agent type: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_levels() {
        assert_eq!(AgentType::Orchestrator.level(), 1);
        assert_eq!(AgentType::Planner.level(), 2);
        assert_eq!(AgentType::Coder.level(), 3);
        assert_eq!(AgentType::Reviewer.level(), 3);
        assert_eq!(AgentType::Tester.level(), 3);
    }

    #[test]
    fn test_agent_type_can_spawn() {
        // Orchestrator can only spawn Planner
        assert!(AgentType::Orchestrator.can_spawn(AgentType::Planner));
        assert!(!AgentType::Orchestrator.can_spawn(AgentType::Coder));

        // Planner can spawn worker agents
        assert!(AgentType::Planner.can_spawn(AgentType::Coder));
        assert!(AgentType::Planner.can_spawn(AgentType::Reviewer));
        assert!(AgentType::Planner.can_spawn(AgentType::Tester));
        assert!(!AgentType::Planner.can_spawn(AgentType::Orchestrator));

        // Workers cannot spawn
        assert!(!AgentType::Coder.can_spawn(AgentType::Coder));
        assert!(!AgentType::Reviewer.can_spawn(AgentType::Tester));
        assert!(!AgentType::Tester.can_spawn(AgentType::Planner));
    }

    #[test]
    fn test_agent_type_is_leaf() {
        assert!(!AgentType::Orchestrator.is_leaf());
        assert!(!AgentType::Planner.is_leaf());
        assert!(AgentType::Coder.is_leaf());
        assert!(AgentType::Reviewer.is_leaf());
        assert!(AgentType::Tester.is_leaf());
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::Orchestrator.to_string(), "orchestrator");
        assert_eq!(AgentType::Planner.to_string(), "planner");
        assert_eq!(AgentType::Coder.to_string(), "coder");
    }

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!(
            "orchestrator".parse::<AgentType>().unwrap(),
            AgentType::Orchestrator
        );
        assert_eq!("PLANNER".parse::<AgentType>().unwrap(), AgentType::Planner);
        assert!("unknown".parse::<AgentType>().is_err());
    }
}
