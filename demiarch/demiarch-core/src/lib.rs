//! Demiarch Core Library
//!
//! This is the shared core library containing all business logic,
//! domain entities, and common functionality used by both
//! the CLI and GUI interfaces.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interfaces;

// Re-export common types and utilities for convenience
pub use domain::*;
pub use infrastructure::db;
pub use infrastructure::llm;
