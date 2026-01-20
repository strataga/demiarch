//! Infrastructure layer - External integrations

pub mod db;
pub mod llm;
pub mod git;
pub mod wasm;

// Re-export common infrastructure types
pub use db::*;
pub use llm::*;
pub use git::*;
pub use wasm::*;