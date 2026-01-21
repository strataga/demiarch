//! Integration tests for encrypted API key storage
//!
//! These tests verify the complete key storage flow including
//! encryption, persistence, and retrieval.

use demiarch_core::domain::security::{
    EncryptedKey, KeyError, KeyRepository, KeyService, MasterKey,
};
use demiarch_core::infrastructure::security::{
    InMemoryKeyRepository, InMemoryMasterKeyRepository, SqliteKeyRepository,
    CREATE_ENCRYPTED_KEYS_TABLE_SQL,
};
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;

/// Create a test database pool
async fn create_test_pool(temp_dir: &TempDir) -> sqlx::SqlitePool {
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Failed to create test pool");

    // Initialize the encrypted_keys table
    sqlx::raw_sql(CREATE_ENCRYPTED_KEYS_TABLE_SQL)
        .execute(&pool)
        .await
        .expect("Failed to create encrypted_keys table");

    pool
}

#[tokio::test]
async fn test_encryption_roundtrip() {
    let master_key = MasterKey::generate();
    let plaintext = "sk-test-api-key-12345-abcdef";

    let encrypted = EncryptedKey::encrypt(
        "test-provider".to_string(),
        plaintext,
        &master_key,
        Some("Test API key".to_string()),
    )
    .unwrap();

    // Verify metadata
    assert_eq!(encrypted.name, "test-provider");
    assert_eq!(encrypted.description, Some("Test API key".to_string()));

    // Decrypt and verify
    let decrypted = encrypted.decrypt(&master_key).unwrap();
    assert_eq!(decrypted.as_str(), plaintext);
}

#[tokio::test]
async fn test_key_service_with_in_memory_repos() {
    let key_repo = Box::new(InMemoryKeyRepository::new());
    let master_key_repo = Box::new(InMemoryMasterKeyRepository::new());
    let service = KeyService::new(key_repo, master_key_repo);

    // Initialize creates master key
    service.initialize().await.unwrap();

    // Store multiple keys
    service
        .store_key("openrouter", "sk-or-v1-test123", Some("OpenRouter API"))
        .await
        .unwrap();
    service
        .store_key("anthropic", "sk-ant-test456", Some("Anthropic API"))
        .await
        .unwrap();

    // Retrieve and verify
    let openrouter_key = service.get_key("openrouter").await.unwrap();
    assert_eq!(openrouter_key.as_str(), "sk-or-v1-test123");

    let anthropic_key = service.get_key("anthropic").await.unwrap();
    assert_eq!(anthropic_key.as_str(), "sk-ant-test456");

    // List keys (should show redacted previews)
    let keys = service.list_keys().await.unwrap();
    assert_eq!(keys.len(), 2);
    for key in &keys {
        assert!(key.preview.starts_with("***"));
    }

    // Update a key
    service
        .update_key("openrouter", "sk-or-v1-updated789")
        .await
        .unwrap();
    let updated_key = service.get_key("openrouter").await.unwrap();
    assert_eq!(updated_key.as_str(), "sk-or-v1-updated789");

    // Delete a key
    service.delete_key("anthropic").await.unwrap();
    assert!(!service.key_exists("anthropic").await.unwrap());

    // Non-existent key should return error
    let result = service.get_key("nonexistent").await;
    assert!(matches!(result, Err(KeyError::NotFound(_))));
}

#[tokio::test]
async fn test_key_service_with_sqlite_repo() {
    let temp_dir = TempDir::new().unwrap();
    let pool = create_test_pool(&temp_dir).await;

    let key_repo = Box::new(SqliteKeyRepository::new(pool));
    let master_key_repo = Box::new(InMemoryMasterKeyRepository::new());
    let service = KeyService::new(key_repo, master_key_repo);

    service.initialize().await.unwrap();

    // Store a key
    let id = service
        .store_key("test-provider", "sk-test-secret", None)
        .await
        .unwrap();

    // Retrieve by name
    let key = service.get_key("test-provider").await.unwrap();
    assert_eq!(key.as_str(), "sk-test-secret");

    // Retrieve by ID
    let key_by_id = service.get_key_by_id(id).await.unwrap();
    assert_eq!(key_by_id.as_str(), "sk-test-secret");

    // Update
    service
        .update_key("test-provider", "sk-test-updated")
        .await
        .unwrap();

    let updated = service.get_key("test-provider").await.unwrap();
    assert_eq!(updated.as_str(), "sk-test-updated");

    // Delete
    service.delete_key("test-provider").await.unwrap();
    assert!(!service.key_exists("test-provider").await.unwrap());
}

#[tokio::test]
async fn test_master_key_rotation() {
    let key_repo = Box::new(InMemoryKeyRepository::new());
    let master_key_repo = Box::new(InMemoryMasterKeyRepository::new());
    let service = KeyService::new(key_repo, master_key_repo);

    service.initialize().await.unwrap();

    // Store some keys
    service
        .store_key("key1", "secret1", None)
        .await
        .unwrap();
    service
        .store_key("key2", "secret2", None)
        .await
        .unwrap();
    service
        .store_key("key3", "secret3", None)
        .await
        .unwrap();

    // Rotate master key
    service.rotate_master_key().await.unwrap();

    // All keys should still be accessible
    assert_eq!(service.get_key("key1").await.unwrap().as_str(), "secret1");
    assert_eq!(service.get_key("key2").await.unwrap().as_str(), "secret2");
    assert_eq!(service.get_key("key3").await.unwrap().as_str(), "secret3");
}

#[tokio::test]
async fn test_duplicate_key_name_rejected() {
    let key_repo = Box::new(InMemoryKeyRepository::new());
    let master_key_repo = Box::new(InMemoryMasterKeyRepository::new());
    let service = KeyService::new(key_repo, master_key_repo);

    service.initialize().await.unwrap();

    service
        .store_key("duplicate-name", "secret1", None)
        .await
        .unwrap();

    let result = service.store_key("duplicate-name", "secret2", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_wrong_master_key_fails_decryption() {
    let master_key1 = MasterKey::generate();
    let master_key2 = MasterKey::generate();

    let encrypted = EncryptedKey::encrypt(
        "test".to_string(),
        "secret",
        &master_key1,
        None,
    )
    .unwrap();

    let result = encrypted.decrypt(&master_key2);
    assert!(matches!(result, Err(KeyError::DecryptionFailed(_))));
}

#[tokio::test]
async fn test_mark_key_used() {
    let key_repo = Box::new(InMemoryKeyRepository::new());
    let master_key_repo = Box::new(InMemoryMasterKeyRepository::new());
    let service = KeyService::new(key_repo, master_key_repo);

    service.initialize().await.unwrap();

    service
        .store_key("test-key", "secret", None)
        .await
        .unwrap();

    // Initially last_used_at should be None
    let keys = service.list_keys().await.unwrap();
    assert!(keys[0].last_used_at.is_none());

    // Mark as used
    service.mark_key_used("test-key").await.unwrap();

    // Now last_used_at should be set
    let keys = service.list_keys().await.unwrap();
    assert!(keys[0].last_used_at.is_some());
}

#[tokio::test]
async fn test_master_key_hex_roundtrip() {
    let original = MasterKey::generate();
    let hex = original.to_hex();
    let restored = MasterKey::from_hex(&hex).unwrap();
    // Compare by re-encoding to hex (since as_bytes is pub(crate))
    assert_eq!(original.to_hex(), restored.to_hex());
}

#[tokio::test]
async fn test_master_key_base64_roundtrip() {
    let original = MasterKey::generate();
    let b64 = original.to_base64();
    let restored = MasterKey::from_base64(&b64).unwrap();
    // Compare by re-encoding to base64 (since as_bytes is pub(crate))
    assert_eq!(original.to_base64(), restored.to_base64());
}

#[tokio::test]
async fn test_invalid_master_key_length() {
    let result = MasterKey::from_bytes(&[0u8; 16]); // Wrong size
    assert!(matches!(result, Err(KeyError::InvalidKeyLength { .. })));
}

#[tokio::test]
async fn test_sqlite_repository_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let pool = create_test_pool(&temp_dir).await;
    let master_key = MasterKey::generate();

    // Store a key
    {
        let repo = SqliteKeyRepository::new(pool.clone());
        let encrypted = EncryptedKey::encrypt(
            "persistent-key".to_string(),
            "persistent-secret",
            &master_key,
            Some("Test persistence".to_string()),
        )
        .unwrap();
        repo.store(&encrypted).await.unwrap();
    }

    // Retrieve from a new repository instance
    {
        let repo = SqliteKeyRepository::new(pool.clone());
        let retrieved = repo.get_by_name("persistent-key").await.unwrap().unwrap();
        assert_eq!(retrieved.name, "persistent-key");

        let decrypted = retrieved.decrypt(&master_key).unwrap();
        assert_eq!(decrypted.as_str(), "persistent-secret");
    }
}
