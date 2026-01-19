//! Demiarch Core Library
//!
//! This crate provides the core functionality for Demiarch, including:
//! - Commands (project, feature, generate, sync, etc.)
//! - Agent system (orchestrator, planner, coder, reviewer, tester)
//! - Storage (SQLite + JSONL export)
//! - LLM integration (OpenRouter API)
//! - Cost management and budget enforcement
//! - Learned skills system
//! - Progressive disclosure context management
//! - Dynamic model routing
//! - Lifecycle hooks

pub mod commands;
pub mod agents;
pub mod storage;
pub mod llm;
pub mod cost;
pub mod skills;
pub mod context;
pub mod routing;
pub mod hooks;
pub mod config;
pub mod error;

pub use error::{Error, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::config::Config;
}
