//! Interfaces layer - Trait definitions

// Trait definitions for external dependencies and cross-boundary communication
// This module will contain repository traits, service interfaces, and other contracts

pub mod repositories;
pub mod services;
pub mod external;

// Re-export common interface types
pub use repositories::*;
pub use services::*;
pub use external::*;