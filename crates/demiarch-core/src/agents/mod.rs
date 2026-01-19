//! Agent system - Russian Doll hierarchical agents

pub mod orchestrator;
pub mod planner;
pub mod coder;
pub mod reviewer;
pub mod tester;
pub mod context;

/// Agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Orchestrator,
    Planner,
    Coder,
    Reviewer,
    Tester,
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
