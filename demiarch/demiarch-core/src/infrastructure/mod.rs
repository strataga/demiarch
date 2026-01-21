//! Infrastructure layer - External integrations

pub mod db;
pub mod git;
pub mod llm;
pub mod wasm;

// Re-export common infrastructure types
pub use db::*;
