//! Repository trait for session persistence
//!
//! This module defines the trait for session storage operations.
//! The trait abstracts over different storage backends (SQLite, etc.).

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;

use super::event::{SessionEvent, SessionEventType};
use super::session::{Session, SessionInfo, SessionStatus};

/// Repository trait for session persistence
///
/// Provides CRUD operations for sessions and session events.
#[async_trait]
pub trait SessionRepositoryTrait: Send + Sync {
    // ========== Session CRUD ==========

    /// Save a new session to the database
    async fn save(&self, session: &Session) -> Result<()>;

    /// Update an existing session
    async fn update(&self, session: &Session) -> Result<()>;

    /// Get a session by ID
    async fn get(&self, session_id: Uuid) -> Result<Option<Session>>;

    /// Get the most recent active session
    async fn get_active(&self) -> Result<Option<Session>>;

    /// Get the most recent session (active or paused)
    async fn get_ongoing(&self) -> Result<Option<Session>>;

    /// List all sessions, ordered by last_activity DESC
    async fn list(&self, limit: Option<i32>) -> Result<Vec<SessionInfo>>;

    /// List sessions by status
    async fn list_by_status(&self, status: SessionStatus) -> Result<Vec<SessionInfo>>;

    /// List sessions for a specific project
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<SessionInfo>>;

    /// Delete a session by ID
    async fn delete(&self, session_id: Uuid) -> Result<bool>;

    /// Delete old completed/abandoned sessions
    async fn delete_older_than(&self, days: i64) -> Result<u64>;

    /// Count sessions by status
    async fn count_by_status(&self, status: SessionStatus) -> Result<i64>;

    // ========== Session Events ==========

    /// Save a session event
    async fn save_event(&self, event: &SessionEvent) -> Result<()>;

    /// Get events for a session
    async fn get_events(&self, session_id: Uuid, limit: Option<i32>) -> Result<Vec<SessionEvent>>;

    /// Get events of a specific type for a session
    async fn get_events_by_type(
        &self,
        session_id: Uuid,
        event_type: SessionEventType,
    ) -> Result<Vec<SessionEvent>>;

    /// Count events for a session
    async fn count_events(&self, session_id: Uuid) -> Result<i64>;

    /// Delete old session events across all sessions
    async fn delete_old_events(&self, days: i64) -> Result<u64>;

    /// Delete events for sessions that have ended (completed or abandoned)
    async fn delete_events_for_ended_sessions(&self) -> Result<u64>;

    /// Delete events for a specific session
    async fn delete_events_for_session(&self, session_id: Uuid) -> Result<u64>;

    /// Count total events across all sessions
    async fn count_all_events(&self) -> Result<i64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify trait is object-safe
    fn _assert_object_safe(_: &dyn SessionRepositoryTrait) {}
}
