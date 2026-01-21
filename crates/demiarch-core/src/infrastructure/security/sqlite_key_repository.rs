//! SQLite-backed encrypted key repository
//!
//! Stores encrypted API keys in an SQLite database with proper
//! parameterized queries to prevent SQL injection.

use crate::domain::security::{EncryptedKey, KeyError, KeyRepository};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// SQL to create the encrypted_keys table
pub const CREATE_ENCRYPTED_KEYS_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS encrypted_keys (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL UNIQUE,
        ciphertext TEXT NOT NULL,
        nonce TEXT NOT NULL,
        description TEXT,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_used_at TIMESTAMP
    );

    CREATE UNIQUE INDEX IF NOT EXISTS idx_encrypted_keys_name ON encrypted_keys(name);
"#;

/// SQL to drop the encrypted_keys table
pub const DROP_ENCRYPTED_KEYS_TABLE_SQL: &str = "DROP TABLE IF EXISTS encrypted_keys;";

/// SQLite-backed implementation of KeyRepository
///
/// This stores encrypted API keys in an SQLite database.
/// The actual encryption/decryption is handled by the domain layer;
/// this repository only persists the encrypted data.
#[derive(Debug, Clone)]
pub struct SqliteKeyRepository {
    pool: SqlitePool,
}

impl SqliteKeyRepository {
    /// Create a new SQLite key repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the database schema for encrypted keys
    ///
    /// This creates the encrypted_keys table if it doesn't exist.
    pub async fn initialize(&self) -> Result<(), KeyError> {
        sqlx::raw_sql(CREATE_ENCRYPTED_KEYS_TABLE_SQL)
            .execute(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to initialize schema: {}", e)))?;
        Ok(())
    }

    /// Parse a database row into an EncryptedKey
    fn row_to_key(row: sqlx::sqlite::SqliteRow) -> Result<EncryptedKey, KeyError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| KeyError::InvalidFormat(format!("Invalid UUID: {}", e)))?;

        Ok(EncryptedKey {
            id,
            name: row.get("name"),
            ciphertext: row.get("ciphertext"),
            nonce: row.get("nonce"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_used_at: row.get("last_used_at"),
        })
    }
}

#[async_trait]
impl KeyRepository for SqliteKeyRepository {
    async fn store(&self, key: &EncryptedKey) -> Result<(), KeyError> {
        sqlx::query(
            r#"
            INSERT INTO encrypted_keys (id, name, ciphertext, nonce, description, created_at, updated_at, last_used_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(key.id.to_string())
        .bind(&key.name)
        .bind(&key.ciphertext)
        .bind(&key.nonce)
        .bind(&key.description)
        .bind(key.created_at)
        .bind(key.updated_at)
        .bind(key.last_used_at)
        .execute(&self.pool)
        .await
        .map_err(|e| KeyError::KeyringError(format!("Failed to store key: {}", e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<EncryptedKey>, KeyError> {
        let row = sqlx::query("SELECT * FROM encrypted_keys WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to get key by id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_key(r)?)),
            None => Ok(None),
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<EncryptedKey>, KeyError> {
        let row = sqlx::query("SELECT * FROM encrypted_keys WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to get key by name: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_key(r)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, key: &EncryptedKey) -> Result<(), KeyError> {
        let result = sqlx::query(
            r#"
            UPDATE encrypted_keys
            SET ciphertext = ?, nonce = ?, description = ?, updated_at = ?, last_used_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&key.ciphertext)
        .bind(&key.nonce)
        .bind(&key.description)
        .bind(key.updated_at)
        .bind(key.last_used_at)
        .bind(key.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| KeyError::KeyringError(format!("Failed to update key: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(KeyError::NotFound(key.id.to_string()));
        }

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), KeyError> {
        sqlx::query("DELETE FROM encrypted_keys WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to delete key: {}", e)))?;

        Ok(())
    }

    async fn delete_by_name(&self, name: &str) -> Result<(), KeyError> {
        sqlx::query("DELETE FROM encrypted_keys WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to delete key by name: {}", e)))?;

        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<EncryptedKey>, KeyError> {
        let rows = sqlx::query("SELECT * FROM encrypted_keys ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to list keys: {}", e)))?;

        rows.into_iter().map(Self::row_to_key).collect()
    }

    async fn exists(&self, name: &str) -> Result<bool, KeyError> {
        let row = sqlx::query("SELECT 1 FROM encrypted_keys WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| KeyError::KeyringError(format!("Failed to check key existence: {}", e)))?;

        Ok(row.is_some())
    }
}

/// In-memory key repository for testing
///
/// This implementation stores keys in memory only.
/// It should NOT be used in production.
#[derive(Debug, Default)]
pub struct InMemoryKeyRepository {
    keys: std::sync::Mutex<std::collections::HashMap<Uuid, EncryptedKey>>,
}

impl InMemoryKeyRepository {
    /// Create a new in-memory repository
    pub fn new() -> Self {
        Self {
            keys: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl KeyRepository for InMemoryKeyRepository {
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
        let mut keys = self.keys.lock().unwrap();
        if let std::collections::hash_map::Entry::Occupied(mut e) = keys.entry(key.id) {
            e.insert(key.clone());
            Ok(())
        } else {
            Err(KeyError::NotFound(key.id.to_string()))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::security::MasterKey;

    #[tokio::test]
    async fn test_in_memory_repository_crud() {
        let repo = InMemoryKeyRepository::new();
        let master_key = MasterKey::generate();

        // Create
        let encrypted =
            EncryptedKey::encrypt("test-key".to_string(), "secret-value", &master_key, None)
                .unwrap();
        repo.store(&encrypted).await.unwrap();

        // Read by ID
        let retrieved = repo.get_by_id(encrypted.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "test-key");

        // Read by name
        let retrieved = repo.get_by_name("test-key").await.unwrap().unwrap();
        assert_eq!(retrieved.id, encrypted.id);

        // Exists
        assert!(repo.exists("test-key").await.unwrap());
        assert!(!repo.exists("nonexistent").await.unwrap());

        // Update
        let mut updated = retrieved.clone();
        updated.update("new-secret-value", &master_key).unwrap();
        repo.update(&updated).await.unwrap();

        let retrieved = repo.get_by_id(encrypted.id).await.unwrap().unwrap();
        assert_ne!(retrieved.ciphertext, encrypted.ciphertext);

        // List all
        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        repo.delete(encrypted.id).await.unwrap();
        assert!(repo.get_by_id(encrypted.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_delete_by_name() {
        let repo = InMemoryKeyRepository::new();
        let master_key = MasterKey::generate();

        let encrypted =
            EncryptedKey::encrypt("test-key".to_string(), "secret-value", &master_key, None)
                .unwrap();
        repo.store(&encrypted).await.unwrap();

        assert!(repo.exists("test-key").await.unwrap());

        repo.delete_by_name("test-key").await.unwrap();

        assert!(!repo.exists("test-key").await.unwrap());
    }
}
