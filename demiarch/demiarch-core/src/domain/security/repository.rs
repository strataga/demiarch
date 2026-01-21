//! Security repository traits
//!
//! Defines the repository interfaces for encrypted key storage.

use super::entity::{EncryptedKey, KeyError, MasterKey, SecurityEntity};
use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

/// Repository for storing and retrieving encrypted API keys
#[async_trait]
pub trait KeyRepository: Send + Sync {
    /// Store an encrypted key
    async fn store(&self, key: &EncryptedKey) -> Result<(), KeyError>;

    /// Retrieve an encrypted key by its unique identifier
    async fn get_by_id(&self, id: Uuid) -> Result<Option<EncryptedKey>, KeyError>;

    /// Retrieve an encrypted key by its name (e.g., "openrouter", "anthropic")
    async fn get_by_name(&self, name: &str) -> Result<Option<EncryptedKey>, KeyError>;

    /// Update an existing encrypted key
    async fn update(&self, key: &EncryptedKey) -> Result<(), KeyError>;

    /// Delete an encrypted key by its identifier
    async fn delete(&self, id: Uuid) -> Result<(), KeyError>;

    /// Delete an encrypted key by its name
    async fn delete_by_name(&self, name: &str) -> Result<(), KeyError>;

    /// List all stored encrypted keys
    async fn list_all(&self) -> Result<Vec<EncryptedKey>, KeyError>;

    /// Check if a key with the given name exists
    async fn exists(&self, name: &str) -> Result<bool, KeyError>;
}

/// Repository for managing the master encryption key via OS keyring
#[async_trait]
pub trait MasterKeyRepository: Send + Sync {
    /// Store the master key in the system keyring
    async fn store(&self, key: &MasterKey) -> Result<(), KeyError>;

    /// Retrieve the master key from the system keyring
    async fn get(&self) -> Result<Option<MasterKey>, KeyError>;

    /// Delete the master key from the system keyring
    async fn delete(&self) -> Result<(), KeyError>;

    /// Check if a master key exists in the keyring
    async fn exists(&self) -> Result<bool, KeyError>;
}

/// Legacy security repository trait (kept for backwards compatibility)
#[async_trait]
pub trait SecurityRepository: Send + Sync {
    async fn create(&self, entity: &SecurityEntity) -> Result<()>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<SecurityEntity>>;
    async fn update(&self, entity: &SecurityEntity) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<SecurityEntity>>;
}
