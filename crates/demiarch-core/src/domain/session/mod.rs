//! Session domain module
//!
//! Provides global session management across multiple projects.
//!
//! # Architecture
//!
//! - **Entities**: `Session`, `SessionEvent`, `SessionInfo`
//! - **Repository**: `SessionRepository` for database operations
//! - **Manager**: `SessionManager` for orchestrating session lifecycle
//!
//! # Features
//!
//! - Global session tracking across all projects
//! - Session lifecycle: create, pause, resume, complete
//! - Event logging for session activities
//! - Automatic session recovery on restart
//! - Cross-project context switching
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::domain::session::{SessionManager, SessionStatus};
//! use sqlx::SqlitePool;
//!
//! // Create manager
//! let manager = SessionManager::new(pool.clone());
//!
//! // Start a new session
//! let session = manager.create(Some(project_id), "Working on auth feature").await?;
//!
//! // Switch projects during session
//! manager.switch_project(&session.id, new_project_id).await?;
//!
//! // Pause session
//! manager.pause(&session.id).await?;
//!
//! // Resume later
//! manager.resume(&session.id).await?;
//!
//! // Complete session
//! manager.complete(&session.id).await?;
//! ```

pub mod event;
pub mod locked_manager;
pub mod manager;
pub mod repository;
pub mod session;

// Re-export main types
pub use event::{SessionEvent, SessionEventType};
pub use locked_manager::LockedSessionManager;
pub use manager::SessionManager;
pub use repository::SessionRepository;
pub use session::{Session, SessionInfo, SessionPhase, SessionStatus};
