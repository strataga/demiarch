//! Security domain services
//!
//! Provides high-level API key management with encryption at rest.

use super::{
    entity::{EncryptedKey, KeyError, MasterKey, SecureString, SecurityEntity},
    repository::{KeyRepository, MasterKeyRepository, SecurityRepository},
};
use anyhow::Result;
use uuid::Uuid;

/// Service for managing encrypted API keys
///
/// This service provides a high-level interface for storing and retrieving
/// API keys with encryption at rest using AES-256-GCM.
///
/// # Security Model
///
/// - API keys are encrypted using AES-256-GCM before storage
/// - A master key is used for encryption/decryption
/// - The master key is stored securely in the OS keyring
/// - Decrypted keys are returned as `SecureString` which is zeroized on drop
/// - Keys are never logged or exposed in debug output
///
/// # Example
///
/// ```ignore
/// let key_service = KeyService::new(key_repo, master_key_repo);
///
/// // Initialize (creates master key if needed)
/// key_service.initialize().await?;
///
/// // Store an API key
/// key_service.store_key("openrouter", "sk-or-v1-xxx", Some("OpenRouter API")).await?;
///
/// // Retrieve and use
/// let api_key = key_service.get_key("openrouter").await?;
/// ```
pub struct KeyService {
    key_repository: Box<dyn KeyRepository>,
    master_key_repository: Box<dyn MasterKeyRepository>,
}

impl KeyService {
    /// Create a new KeyService
    pub fn new(
        key_repository: Box<dyn KeyRepository>,
        master_key_repository: Box<dyn MasterKeyRepository>,
    ) -> Self {
        Self {
            key_repository,
            master_key_repository,
        }
    }

    /// Initialize the key service
    ///
    /// Creates a new master key if one doesn't exist in the keyring.
    /// This should be called once during application startup.
    pub async fn initialize(&self) -> Result<(), KeyError> {
        if !self.master_key_repository.exists().await? {
            let master_key = MasterKey::generate();
            self.master_key_repository.store(&master_key).await?;
            tracing::info!("Generated and stored new master encryption key");
        }
        Ok(())
    }

    /// Get the master key, initializing if necessary
    async fn get_master_key(&self) -> Result<MasterKey, KeyError> {
        match self.master_key_repository.get().await? {
            Some(key) => Ok(key),
            None => {
                // Auto-initialize if master key doesn't exist
                self.initialize().await?;
                self.master_key_repository.get().await?.ok_or_else(|| {
                    KeyError::KeyringError(
                        "Failed to retrieve master key after initialization".to_string(),
                    )
                })
            }
        }
    }

    /// Store a new API key
    ///
    /// # Arguments
    ///
    /// * `name` - A unique identifier for the key (e.g., "openrouter", "anthropic")
    /// * `plaintext` - The actual API key value
    /// * `description` - Optional description of what this key is for
    ///
    /// # Returns
    ///
    /// The UUID of the stored key
    pub async fn store_key(
        &self,
        name: &str,
        plaintext: &str,
        description: Option<&str>,
    ) -> Result<Uuid, KeyError> {
        // Check if key with this name already exists
        if self.key_repository.exists(name).await? {
            return Err(KeyError::EncryptionFailed(format!(
                "A key with name '{}' already exists. Use update_key to modify it.",
                name
            )));
        }

        let master_key = self.get_master_key().await?;
        let encrypted = EncryptedKey::encrypt(
            name.to_string(),
            plaintext,
            &master_key,
            description.map(String::from),
        )?;

        let id = encrypted.id;
        self.key_repository.store(&encrypted).await?;

        tracing::info!(key_name = %name, "Stored encrypted API key");
        Ok(id)
    }

    /// Retrieve a decrypted API key by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the key to retrieve
    ///
    /// # Returns
    ///
    /// A `SecureString` containing the decrypted key, which will be
    /// securely zeroed when dropped.
    pub async fn get_key(&self, name: &str) -> Result<SecureString, KeyError> {
        let encrypted = self
            .key_repository
            .get_by_name(name)
            .await?
            .ok_or_else(|| KeyError::NotFound(name.to_string()))?;

        let master_key = self.get_master_key().await?;
        encrypted.decrypt(&master_key)
    }

    /// Retrieve a decrypted API key by UUID
    pub async fn get_key_by_id(&self, id: Uuid) -> Result<SecureString, KeyError> {
        let encrypted = self
            .key_repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| KeyError::NotFound(id.to_string()))?;

        let master_key = self.get_master_key().await?;
        encrypted.decrypt(&master_key)
    }

    /// Update an existing API key
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the key to update
    /// * `new_plaintext` - The new API key value
    pub async fn update_key(&self, name: &str, new_plaintext: &str) -> Result<(), KeyError> {
        let mut encrypted = self
            .key_repository
            .get_by_name(name)
            .await?
            .ok_or_else(|| KeyError::NotFound(name.to_string()))?;

        let master_key = self.get_master_key().await?;
        encrypted.update(new_plaintext, &master_key)?;
        self.key_repository.update(&encrypted).await?;

        tracing::info!(key_name = %name, "Updated encrypted API key");
        Ok(())
    }

    /// Delete an API key by name
    pub async fn delete_key(&self, name: &str) -> Result<(), KeyError> {
        self.key_repository.delete_by_name(name).await?;
        tracing::info!(key_name = %name, "Deleted API key");
        Ok(())
    }

    /// Delete an API key by UUID
    pub async fn delete_key_by_id(&self, id: Uuid) -> Result<(), KeyError> {
        self.key_repository.delete(id).await?;
        tracing::info!(key_id = %id, "Deleted API key");
        Ok(())
    }

    /// List all stored keys (metadata only, not decrypted values)
    pub async fn list_keys(&self) -> Result<Vec<KeyInfo>, KeyError> {
        let master_key = self.get_master_key().await?;
        let keys = self.key_repository.list_all().await?;

        Ok(keys
            .into_iter()
            .map(|k| KeyInfo {
                id: k.id,
                name: k.name.clone(),
                description: k.description.clone(),
                preview: k.redacted_preview(&master_key),
                created_at: k.created_at,
                updated_at: k.updated_at,
                last_used_at: k.last_used_at,
            })
            .collect())
    }

    /// Check if a key exists
    pub async fn key_exists(&self, name: &str) -> Result<bool, KeyError> {
        self.key_repository.exists(name).await
    }

    /// Mark a key as having been used (updates last_used_at)
    pub async fn mark_key_used(&self, name: &str) -> Result<(), KeyError> {
        let mut encrypted = self
            .key_repository
            .get_by_name(name)
            .await?
            .ok_or_else(|| KeyError::NotFound(name.to_string()))?;

        encrypted.mark_used();
        self.key_repository.update(&encrypted).await?;
        Ok(())
    }

    /// Rotate the master encryption key
    ///
    /// This re-encrypts all stored keys with a new master key.
    /// Should be done periodically for security best practices.
    pub async fn rotate_master_key(&self) -> Result<(), KeyError> {
        let old_master_key = self.get_master_key().await?;
        let new_master_key = MasterKey::generate();

        // Get all keys
        let keys = self.key_repository.list_all().await?;

        // Re-encrypt each key with the new master key
        for mut key in keys {
            // Decrypt with old key
            let plaintext = key.decrypt(&old_master_key)?;

            // Re-encrypt with new key
            key.update(plaintext.as_str(), &new_master_key)?;

            // Update in repository
            self.key_repository.update(&key).await?;
        }

        // Store the new master key
        self.master_key_repository.store(&new_master_key).await?;

        tracing::info!("Rotated master encryption key and re-encrypted all API keys");
        Ok(())
    }

    /// Delete the master key (WARNING: all encrypted keys become unrecoverable)
    ///
    /// This should only be used when completely resetting the key store.
    pub async fn destroy_master_key(&self) -> Result<(), KeyError> {
        self.master_key_repository.delete().await?;
        tracing::warn!(
            "Destroyed master encryption key - all encrypted keys are now unrecoverable"
        );
        Ok(())
    }
}

/// Information about a stored key (without the actual value)
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// Unique identifier
    pub id: Uuid,
    /// Key name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Redacted preview (e.g., "***1234")
    pub preview: String,
    /// When the key was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the key was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// When the key was last used
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Legacy security service (kept for backwards compatibility)
pub struct SecurityService {
    repository: Box<dyn SecurityRepository>,
}

impl SecurityService {
    pub fn new(repository: Box<dyn SecurityRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_security(&self, name: String) -> Result<SecurityEntity> {
        let entity = SecurityEntity {
            id: uuid::Uuid::new_v4(),
            name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repository.create(&entity).await?;
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock KeyRepository for testing
    struct MockKeyRepository {
        keys: Mutex<HashMap<Uuid, EncryptedKey>>,
    }

    impl MockKeyRepository {
        fn new() -> Self {
            Self {
                keys: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl KeyRepository for MockKeyRepository {
        async fn store(&self, key: &EncryptedKey) -> Result<(), KeyError> {
            self.keys.lock().unwrap().insert(key.id, key.clone());
            Ok(())
        }

        async fn get_by_id(&self, id: Uuid) -> Result<Option<EncryptedKey>, KeyError> {
            Ok(self.keys.lock().unwrap().get(&id).cloned())
        }

        async fn get_by_name(&self, name: &str) -> Result<Option<EncryptedKey>, KeyError> {
            Ok(self
                .keys
                .lock()
                .unwrap()
                .values()
                .find(|k| k.name == name)
                .cloned())
        }

        async fn update(&self, key: &EncryptedKey) -> Result<(), KeyError> {
            self.keys.lock().unwrap().insert(key.id, key.clone());
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<(), KeyError> {
            self.keys.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn delete_by_name(&self, name: &str) -> Result<(), KeyError> {
            let mut keys = self.keys.lock().unwrap();
            keys.retain(|_, v| v.name != name);
            Ok(())
        }

        async fn list_all(&self) -> Result<Vec<EncryptedKey>, KeyError> {
            Ok(self.keys.lock().unwrap().values().cloned().collect())
        }

        async fn exists(&self, name: &str) -> Result<bool, KeyError> {
            Ok(self.keys.lock().unwrap().values().any(|k| k.name == name))
        }
    }

    // Mock MasterKeyRepository for testing
    struct MockMasterKeyRepository {
        key: Mutex<Option<MasterKey>>,
    }

    impl MockMasterKeyRepository {
        fn new() -> Self {
            Self {
                key: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl MasterKeyRepository for MockMasterKeyRepository {
        async fn store(&self, key: &MasterKey) -> Result<(), KeyError> {
            *self.key.lock().unwrap() = Some(key.clone());
            Ok(())
        }

        async fn get(&self) -> Result<Option<MasterKey>, KeyError> {
            Ok(self.key.lock().unwrap().clone())
        }

        async fn delete(&self) -> Result<(), KeyError> {
            *self.key.lock().unwrap() = None;
            Ok(())
        }

        async fn exists(&self) -> Result<bool, KeyError> {
            Ok(self.key.lock().unwrap().is_some())
        }
    }

    #[tokio::test]
    async fn test_key_service_initialize() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        // Second initialize should be a no-op
        service.initialize().await.unwrap();
    }

    #[tokio::test]
    async fn test_store_and_retrieve_key() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        let api_key = "sk-test-12345";
        service
            .store_key("test-provider", api_key, Some("Test key"))
            .await
            .unwrap();

        let retrieved = service.get_key("test-provider").await.unwrap();
        assert_eq!(retrieved.as_str(), api_key);
    }

    #[tokio::test]
    async fn test_update_key() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        service
            .store_key("test-provider", "old-key", None)
            .await
            .unwrap();

        service
            .update_key("test-provider", "new-key")
            .await
            .unwrap();

        let retrieved = service.get_key("test-provider").await.unwrap();
        assert_eq!(retrieved.as_str(), "new-key");
    }

    #[tokio::test]
    async fn test_delete_key() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        service
            .store_key("test-provider", "test-key", None)
            .await
            .unwrap();

        assert!(service.key_exists("test-provider").await.unwrap());

        service.delete_key("test-provider").await.unwrap();

        assert!(!service.key_exists("test-provider").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_keys() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        service
            .store_key("provider-1", "key-1", Some("First key"))
            .await
            .unwrap();
        service
            .store_key("provider-2", "key-2", Some("Second key"))
            .await
            .unwrap();

        let keys = service.list_keys().await.unwrap();
        assert_eq!(keys.len(), 2);

        // Verify keys don't contain actual values
        for key in &keys {
            assert!(key.preview.starts_with("***"));
        }
    }

    #[tokio::test]
    async fn test_duplicate_key_name_fails() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        service
            .store_key("test-provider", "key-1", None)
            .await
            .unwrap();

        let result = service.store_key("test-provider", "key-2", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rotate_master_key() {
        let key_repo = Box::new(MockKeyRepository::new());
        let master_key_repo = Box::new(MockMasterKeyRepository::new());
        let service = KeyService::new(key_repo, master_key_repo);

        service.initialize().await.unwrap();

        // Store some keys
        service
            .store_key("provider-1", "secret-1", None)
            .await
            .unwrap();
        service
            .store_key("provider-2", "secret-2", None)
            .await
            .unwrap();

        // Rotate the master key
        service.rotate_master_key().await.unwrap();

        // Keys should still be accessible
        let key1 = service.get_key("provider-1").await.unwrap();
        let key2 = service.get_key("provider-2").await.unwrap();

        assert_eq!(key1.as_str(), "secret-1");
        assert_eq!(key2.as_str(), "secret-2");
    }
}
