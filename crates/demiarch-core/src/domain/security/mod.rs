//! Security domain module
//!
//! Provides encrypted API key storage with AES-256-GCM encryption at rest.
//!
//! # Architecture
//!
//! - **Entities**: `MasterKey`, `EncryptedKey`, `SecureString`
//! - **Repository Traits**: `KeyRepository`, `MasterKeyRepository`
//! - **Services**: `KeyService` for high-level key management
//!
//! # Security Features
//!
//! - AES-256-GCM authenticated encryption
//! - Master key stored in OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
//! - Secure memory handling with zeroize-on-drop
//! - Unique nonce per encryption operation
//! - Master key rotation support
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::security::{KeyService, MasterKey};
//! use demiarch_core::infrastructure::security::{KeyringMasterKeyRepository, SqliteKeyRepository};
//!
//! // Create repositories
//! let key_repo = SqliteKeyRepository::new(pool);
//! let master_key_repo = KeyringMasterKeyRepository::new();
//!
//! // Create service
//! let key_service = KeyService::new(
//!     Box::new(key_repo),
//!     Box::new(master_key_repo),
//! );
//!
//! // Initialize (creates master key if needed)
//! key_service.initialize().await?;
//!
//! // Store an API key
//! key_service.store_key("openrouter", "sk-or-v1-xxx", Some("OpenRouter API")).await?;
//!
//! // Retrieve when needed
//! let api_key = key_service.get_key("openrouter").await?;
//! // api_key is a SecureString that will be zeroized when dropped
//! ```

pub mod entity;
pub mod event;
pub mod repository;
pub mod service;

// Re-export entity types
pub use entity::{EncryptedKey, KeyError, MasterKey, SecureString, SecurityEntity};

// Re-export event types
pub use event::{SecurityEvent, SecurityEventType};

// Re-export repository traits
pub use repository::{KeyRepository, MasterKeyRepository, SecurityRepository};

// Re-export service types
pub use service::{KeyInfo, KeyService, SecurityService};
