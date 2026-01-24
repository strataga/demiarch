//! GUI API Module
//!
//! Provides a clean async interface for GUI applications to interact with
//! demiarch-core functionality. This module handles database connections
//! and translates domain types to DTOs suitable for serialization.

pub mod projects;
pub mod features;
pub mod sessions;
pub mod costs;
pub mod health;

use crate::storage::{Database, DatabaseConfig};
use crate::Result;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

/// Global database instance for the API
static DATABASE: OnceLock<Arc<RwLock<Option<Database>>>> = OnceLock::new();

/// Get or initialize the database connection
pub async fn get_database() -> Result<Database> {
    let db_holder = DATABASE.get_or_init(|| Arc::new(RwLock::new(None)));

    // Check if already initialized
    {
        let guard = db_holder.read().await;
        if let Some(db) = guard.as_ref() {
            return Ok(db.clone());
        }
    }

    // Initialize database
    let mut guard = db_holder.write().await;
    if guard.is_none() {
        let db = Database::default()
            .await
            .map_err(|e| crate::Error::DatabaseError(sqlx::Error::Configuration(e.into())))?;
        *guard = Some(db);
    }

    Ok(guard.as_ref().unwrap().clone())
}

/// Initialize the database with a specific path
pub async fn init_database(path: PathBuf) -> Result<()> {
    let db_holder = DATABASE.get_or_init(|| Arc::new(RwLock::new(None)));
    let mut guard = db_holder.write().await;

    let config = DatabaseConfig::with_path(path);
    let db = Database::new(config)
        .await
        .map_err(|e| crate::Error::DatabaseError(sqlx::Error::Configuration(e.into())))?;
    *guard = Some(db);

    Ok(())
}

/// Check if the database is initialized
pub async fn is_initialized() -> bool {
    if let Some(holder) = DATABASE.get() {
        let guard = holder.read().await;
        guard.is_some()
    } else {
        false
    }
}
