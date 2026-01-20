//! Domain layer - Business logic and entities

pub mod agents;
pub mod code_generation;
pub mod context;
pub mod cost;
pub mod plugins;
pub mod projects;
pub mod recovery;
pub mod security;
pub mod skills;

// Re-export specific types to avoid naming conflicts
pub use agents::{Agent, AgentRepository, AgentService, AgentType};
pub use projects::{Project, ProjectName, ProjectRepository, ProjectService};
