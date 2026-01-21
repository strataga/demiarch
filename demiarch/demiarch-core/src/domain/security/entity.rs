//! Security domain entities
//!
//! Provides encrypted key storage with AES-256-GCM encryption at rest.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use chrono::{DateTime, Utc};
use rand_chacha::rand_core::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Size of AES-256 key in bytes
const AES_KEY_SIZE: usize = 32;

/// Size of AES-GCM nonce in bytes
const NONCE_SIZE: usize = 12;

/// Errors that can occur during key operations
#[derive(Debug, Error)]
pub enum KeyError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Keyring error: {0}")]
    KeyringError(String),

    #[error("Invalid key format: {0}")]
    InvalidFormat(String),
}

/// A master encryption key that is securely zeroed on drop
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterKey {
    bytes: [u8; AES_KEY_SIZE],
}

impl MasterKey {
    /// Generate a new random master key
    pub fn generate() -> Self {
        let mut bytes = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut bytes);
        Self { bytes }
    }

    /// Create a master key from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyError> {
        if bytes.len() != AES_KEY_SIZE {
            return Err(KeyError::InvalidKeyLength {
                expected: AES_KEY_SIZE,
                actual: bytes.len(),
            });
        }
        let mut key_bytes = [0u8; AES_KEY_SIZE];
        key_bytes.copy_from_slice(bytes);
        Ok(Self { bytes: key_bytes })
    }

    /// Create a master key from hex-encoded string
    pub fn from_hex(hex: &str) -> Result<Self, KeyError> {
        let bytes = hex::decode(hex).map_err(|e| KeyError::InvalidFormat(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Create a master key from base64-encoded string
    pub fn from_base64(b64: &str) -> Result<Self, KeyError> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let bytes = STANDARD
            .decode(b64)
            .map_err(|e| KeyError::InvalidFormat(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Export key as hex string (for storage in keyring)
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Export key as base64 string
    pub fn to_base64(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD.encode(&self.bytes)
    }

    /// Get the raw key bytes (use carefully)
    pub(crate) fn as_bytes(&self) -> &[u8; AES_KEY_SIZE] {
        &self.bytes
    }
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasterKey")
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

/// An encrypted API key with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKey {
    /// Unique identifier for this key
    pub id: Uuid,

    /// Human-readable name for the key (e.g., "openrouter", "anthropic")
    pub name: String,

    /// The encrypted key data (base64 encoded)
    pub ciphertext: String,

    /// The nonce used for encryption (base64 encoded)
    pub nonce: String,

    /// Optional description or notes about this key
    pub description: Option<String>,

    /// When this key was created
    pub created_at: DateTime<Utc>,

    /// When this key was last updated
    pub updated_at: DateTime<Utc>,

    /// When this key was last used
    pub last_used_at: Option<DateTime<Utc>>,
}

impl EncryptedKey {
    /// Encrypt a plaintext API key using the master key
    pub fn encrypt(
        name: String,
        plaintext: &str,
        master_key: &MasterKey,
        description: Option<String>,
    ) -> Result<Self, KeyError> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher and encrypt
        let cipher = Aes256Gcm::new_from_slice(master_key.as_bytes())
            .map_err(|e| KeyError::EncryptionFailed(e.to_string()))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| KeyError::EncryptionFailed(e.to_string()))?;

        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            ciphertext: STANDARD.encode(&ciphertext),
            nonce: STANDARD.encode(&nonce_bytes),
            description,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        })
    }

    /// Decrypt the API key using the master key
    ///
    /// Returns a SecureString that is zeroized on drop
    pub fn decrypt(&self, master_key: &MasterKey) -> Result<SecureString, KeyError> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        // Decode ciphertext and nonce
        let ciphertext = STANDARD
            .decode(&self.ciphertext)
            .map_err(|e| KeyError::DecryptionFailed(format!("Invalid ciphertext: {}", e)))?;

        let nonce_bytes = STANDARD
            .decode(&self.nonce)
            .map_err(|e| KeyError::DecryptionFailed(format!("Invalid nonce: {}", e)))?;

        if nonce_bytes.len() != NONCE_SIZE {
            return Err(KeyError::DecryptionFailed(format!(
                "Invalid nonce length: expected {}, got {}",
                NONCE_SIZE,
                nonce_bytes.len()
            )));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher and decrypt
        let cipher = Aes256Gcm::new_from_slice(master_key.as_bytes())
            .map_err(|e| KeyError::DecryptionFailed(e.to_string()))?;

        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
            KeyError::DecryptionFailed(
                "Decryption failed (invalid key or corrupted data)".to_string(),
            )
        })?;

        let decrypted = String::from_utf8(plaintext)
            .map_err(|e| KeyError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))?;

        Ok(SecureString::new(decrypted))
    }

    /// Update the encrypted value
    pub fn update(&mut self, plaintext: &str, master_key: &MasterKey) -> Result<(), KeyError> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        // Generate new nonce for each encryption
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(master_key.as_bytes())
            .map_err(|e| KeyError::EncryptionFailed(e.to_string()))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| KeyError::EncryptionFailed(e.to_string()))?;

        self.ciphertext = STANDARD.encode(&ciphertext);
        self.nonce = STANDARD.encode(&nonce_bytes);
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Mark this key as having been used
    pub fn mark_used(&mut self) {
        self.last_used_at = Some(Utc::now());
    }

    /// Get a redacted preview of the decrypted key (last 4 chars)
    pub fn redacted_preview(&self, master_key: &MasterKey) -> String {
        match self.decrypt(master_key) {
            Ok(plaintext) => {
                let s = plaintext.as_str();
                if s.len() > 4 {
                    format!("***{}", &s[s.len() - 4..])
                } else {
                    "***".to_string()
                }
            }
            Err(_) => "[decryption failed]".to_string(),
        }
    }
}

/// A string that is securely zeroed when dropped
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureString {
    inner: String,
}

impl SecureString {
    /// Create a new secure string
    pub fn new(s: String) -> Self {
        Self { inner: s }
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Consume and return the inner string
    ///
    /// WARNING: The returned String will NOT be zeroized.
    /// Only use this when you need to pass the value to an external API.
    pub fn into_inner(self) -> String {
        // We can't prevent the inner string from being copied here,
        // but at least our copy will be zeroized
        self.inner.clone()
    }
}

impl std::fmt::Debug for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureString")
            .field("inner", &"[REDACTED]")
            .finish()
    }
}

impl AsRef<str> for SecureString {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}

/// Legacy security entity (kept for backwards compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEntity {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_generation() {
        let key1 = MasterKey::generate();
        let key2 = MasterKey::generate();

        // Keys should be different
        assert_ne!(key1.as_bytes(), key2.as_bytes());

        // Key should be correct length
        assert_eq!(key1.as_bytes().len(), AES_KEY_SIZE);
    }

    #[test]
    fn test_master_key_from_bytes() {
        let bytes = [42u8; AES_KEY_SIZE];
        let key = MasterKey::from_bytes(&bytes).unwrap();
        assert_eq!(key.as_bytes(), &bytes);
    }

    #[test]
    fn test_master_key_invalid_length() {
        let bytes = [42u8; 16]; // Wrong size
        let result = MasterKey::from_bytes(&bytes);
        assert!(matches!(result, Err(KeyError::InvalidKeyLength { .. })));
    }

    #[test]
    fn test_master_key_hex_roundtrip() {
        let key = MasterKey::generate();
        let hex = key.to_hex();
        let restored = MasterKey::from_hex(&hex).unwrap();
        assert_eq!(key.as_bytes(), restored.as_bytes());
    }

    #[test]
    fn test_master_key_base64_roundtrip() {
        let key = MasterKey::generate();
        let b64 = key.to_base64();
        let restored = MasterKey::from_base64(&b64).unwrap();
        assert_eq!(key.as_bytes(), restored.as_bytes());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let master_key = MasterKey::generate();
        let plaintext = "sk-test-api-key-12345";

        let encrypted = EncryptedKey::encrypt(
            "test-key".to_string(),
            plaintext,
            &master_key,
            Some("Test API key".to_string()),
        )
        .unwrap();

        let decrypted = encrypted.decrypt(&master_key).unwrap();
        assert_eq!(decrypted.as_str(), plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let master_key1 = MasterKey::generate();
        let master_key2 = MasterKey::generate();
        let plaintext = "sk-test-api-key-12345";

        let encrypted =
            EncryptedKey::encrypt("test-key".to_string(), plaintext, &master_key1, None).unwrap();

        let result = encrypted.decrypt(&master_key2);
        assert!(matches!(result, Err(KeyError::DecryptionFailed(_))));
    }

    #[test]
    fn test_update_key() {
        let master_key = MasterKey::generate();
        let original = "sk-original-key";
        let updated = "sk-updated-key";

        let mut encrypted =
            EncryptedKey::encrypt("test-key".to_string(), original, &master_key, None).unwrap();

        let original_nonce = encrypted.nonce.clone();

        encrypted.update(updated, &master_key).unwrap();

        // Nonce should be different after update
        assert_ne!(encrypted.nonce, original_nonce);

        // Should decrypt to new value
        let decrypted = encrypted.decrypt(&master_key).unwrap();
        assert_eq!(decrypted.as_str(), updated);
    }

    #[test]
    fn test_redacted_preview() {
        let master_key = MasterKey::generate();
        let plaintext = "sk-test-12345";

        let encrypted =
            EncryptedKey::encrypt("test-key".to_string(), plaintext, &master_key, None).unwrap();

        let preview = encrypted.redacted_preview(&master_key);
        assert_eq!(preview, "***2345");
    }

    #[test]
    fn test_secure_string_debug_redacted() {
        let secure = SecureString::new("secret".to_string());
        let debug = format!("{:?}", secure);
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn test_master_key_debug_redacted() {
        let key = MasterKey::generate();
        let debug = format!("{:?}", key);
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_encrypted_key_metadata() {
        let master_key = MasterKey::generate();
        let mut encrypted = EncryptedKey::encrypt(
            "test-key".to_string(),
            "secret",
            &master_key,
            Some("Description".to_string()),
        )
        .unwrap();

        assert_eq!(encrypted.name, "test-key");
        assert_eq!(encrypted.description, Some("Description".to_string()));
        assert!(encrypted.last_used_at.is_none());

        encrypted.mark_used();
        assert!(encrypted.last_used_at.is_some());
    }
}
