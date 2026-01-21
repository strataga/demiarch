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
//! - Encrypted API key storage (AES-256-GCM)

pub mod agents;
pub mod commands;
pub mod config;
pub mod context;
pub mod cost;
pub mod domain;
pub mod error;
pub mod hooks;
pub mod infrastructure;
pub mod llm;
pub mod routing;
pub mod skills;
pub mod storage;

pub use error::{Error, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::config::Config;
    pub use crate::error::{Error, Result};
}

/// Re-export security types for convenient access
pub mod security {
    pub use crate::domain::security::{
        EncryptedKey, KeyError, KeyInfo, KeyRepository, KeyService, MasterKey, MasterKeyRepository,
        SecureString,
    };
    pub use crate::infrastructure::security::{
        CREATE_ENCRYPTED_KEYS_TABLE_SQL, InMemoryKeyRepository, InMemoryMasterKeyRepository,
        KeyringMasterKeyRepository, SqliteKeyRepository,
    };
}

#[cfg(test)]
mod commands_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod error_tests;
