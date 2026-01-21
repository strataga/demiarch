//! OS Keyring integration for master key storage
//!
//! Stores the master encryption key securely in the operating system's
//! credential store (e.g., macOS Keychain, Windows Credential Manager,
//! Linux Secret Service).

use crate::domain::security::{KeyError, MasterKey, MasterKeyRepository};
use async_trait::async_trait;
use keyring::Entry;

/// Service name used for keyring storage
const KEYRING_SERVICE: &str = "demiarch";

/// Default username for keyring entries
const KEYRING_USER: &str = "master-encryption-key";

/// OS keyring-based master key repository
///
/// This implementation stores the master encryption key in the operating
/// system's secure credential store.
///
/// # Platform Support
///
/// - **macOS**: Uses Keychain Services
/// - **Windows**: Uses Windows Credential Manager
/// - **Linux**: Uses Secret Service API (requires a secret service daemon)
///
/// # Security
///
/// The master key is stored as a hex-encoded string in the keyring.
/// Access to the keyring typically requires user authentication
/// (e.g., password, biometrics) depending on OS configuration.
#[derive(Debug, Clone)]
pub struct KeyringMasterKeyRepository {
    service: String,
    user: String,
}

impl Default for KeyringMasterKeyRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringMasterKeyRepository {
    /// Create a new keyring repository with default service/user
    pub fn new() -> Self {
        Self {
            service: KEYRING_SERVICE.to_string(),
            user: KEYRING_USER.to_string(),
        }
    }

    /// Create a keyring repository with custom service/user names
    ///
    /// This can be useful for testing or multi-tenant scenarios.
    pub fn with_names(service: &str, user: &str) -> Self {
        Self {
            service: service.to_string(),
            user: user.to_string(),
        }
    }

    /// Get the keyring entry
    fn entry(&self) -> Result<Entry, KeyError> {
        Entry::new(&self.service, &self.user)
            .map_err(|e| KeyError::KeyringError(format!("Failed to create keyring entry: {}", e)))
    }
}

#[async_trait]
impl MasterKeyRepository for KeyringMasterKeyRepository {
    async fn store(&self, key: &MasterKey) -> Result<(), KeyError> {
        let entry = self.entry()?;
        let hex_key = key.to_hex();

        // keyring operations are blocking, so we spawn a blocking task
        tokio::task::spawn_blocking(move || {
            entry
                .set_password(&hex_key)
                .map_err(|e| KeyError::KeyringError(format!("Failed to store master key: {}", e)))
        })
        .await
        .map_err(|e| KeyError::KeyringError(format!("Task join error: {}", e)))?
    }

    async fn get(&self) -> Result<Option<MasterKey>, KeyError> {
        let entry = self.entry()?;

        let result = tokio::task::spawn_blocking(move || entry.get_password())
            .await
            .map_err(|e| KeyError::KeyringError(format!("Task join error: {}", e)))?;

        match result {
            Ok(hex_key) => {
                let key = MasterKey::from_hex(&hex_key)?;
                Ok(Some(key))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(KeyError::KeyringError(format!(
                "Failed to retrieve master key: {}",
                e
            ))),
        }
    }

    async fn delete(&self) -> Result<(), KeyError> {
        let entry = self.entry()?;

        tokio::task::spawn_blocking(move || match entry.delete_password() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(KeyError::KeyringError(format!(
                "Failed to delete master key: {}",
                e
            ))),
        })
        .await
        .map_err(|e| KeyError::KeyringError(format!("Task join error: {}", e)))?
    }

    async fn exists(&self) -> Result<bool, KeyError> {
        let entry = self.entry()?;

        tokio::task::spawn_blocking(move || match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(KeyError::KeyringError(format!(
                "Failed to check master key existence: {}",
                e
            ))),
        })
        .await
        .map_err(|e| KeyError::KeyringError(format!("Task join error: {}", e)))?
    }
}

/// In-memory master key repository for testing
///
/// This implementation stores the master key in memory only.
/// It should NOT be used in production.
#[derive(Debug, Default)]
pub struct InMemoryMasterKeyRepository {
    key: std::sync::Mutex<Option<MasterKey>>,
}

impl InMemoryMasterKeyRepository {
    /// Create a new in-memory repository
    pub fn new() -> Self {
        Self {
            key: std::sync::Mutex::new(None),
        }
    }
}

#[async_trait]
impl MasterKeyRepository for InMemoryMasterKeyRepository {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_repository() {
        let repo = InMemoryMasterKeyRepository::new();

        // Initially empty
        assert!(!repo.exists().await.unwrap());
        assert!(repo.get().await.unwrap().is_none());

        // Store a key
        let key = MasterKey::generate();
        repo.store(&key).await.unwrap();

        // Should exist now
        assert!(repo.exists().await.unwrap());

        // Retrieve and verify
        let retrieved = repo.get().await.unwrap().unwrap();
        assert_eq!(key.as_bytes(), retrieved.as_bytes());

        // Delete
        repo.delete().await.unwrap();
        assert!(!repo.exists().await.unwrap());
    }

    // Note: Keyring tests require a running secret service and are
    // typically run manually or in integration test environments
    #[tokio::test]
    #[ignore = "Requires OS keyring access"]
    async fn test_keyring_repository() {
        let repo = KeyringMasterKeyRepository::with_names("demiarch-test", "test-key");

        // Clean up any existing test key
        let _ = repo.delete().await;

        // Initially empty
        assert!(!repo.exists().await.unwrap());

        // Store a key
        let key = MasterKey::generate();
        repo.store(&key).await.unwrap();

        // Should exist now
        assert!(repo.exists().await.unwrap());

        // Retrieve and verify
        let retrieved = repo.get().await.unwrap().unwrap();
        assert_eq!(key.as_bytes(), retrieved.as_bytes());

        // Clean up
        repo.delete().await.unwrap();
        assert!(!repo.exists().await.unwrap());
    }
}
