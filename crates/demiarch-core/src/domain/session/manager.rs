//! Session manager for orchestrating session lifecycle
//!
//! Provides high-level operations for managing sessions, including
//! creation, lifecycle transitions, and context switching.

use super::event::SessionEvent;
use super::repository::SessionRepository;
use super::session::{Session, SessionInfo, SessionPhase, SessionStatus};
use crate::error::{Error, Result};
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

/// Manager for session lifecycle operations
#[derive(Debug, Clone)]
pub struct SessionManager {
    repository: SessionRepository,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            repository: SessionRepository::new(pool),
        }
    }

    /// Get the underlying repository
    pub fn repository(&self) -> &SessionRepository {
        &self.repository
    }

    // ========== Session Lifecycle ==========

    /// Create a new session
    ///
    /// Automatically pauses any existing active session before creating the new one.
    pub async fn create(
        &self,
        project_id: Option<Uuid>,
        feature_id: Option<Uuid>,
        description: Option<String>,
    ) -> Result<Session> {
        // Pause any existing active session
        if let Some(mut active) = self.repository.get_active().await? {
            info!(session_id = %active.id, "Pausing existing active session");
            active.pause();
            self.repository.update(&active).await?;
            self.repository
                .save_event(&SessionEvent::paused(active.id))
                .await?;
        }

        // Create new session
        let session = Session::new(project_id, feature_id, description);
        self.repository.save(&session).await?;
        self.repository
            .save_event(&SessionEvent::started(session.id))
            .await?;

        info!(
            session_id = %session.id,
            project_id = ?project_id,
            "Created new session"
        );

        Ok(session)
    }

    /// Get a session by ID
    pub async fn get(&self, session_id: Uuid) -> Result<Option<Session>> {
        self.repository.get(session_id).await
    }

    /// Get the current active session
    pub async fn get_active(&self) -> Result<Option<Session>> {
        self.repository.get_active().await
    }

    /// Get or create a session
    ///
    /// Returns the active session if one exists, otherwise creates a new one.
    pub async fn get_or_create(
        &self,
        project_id: Option<Uuid>,
        description: Option<String>,
    ) -> Result<Session> {
        if let Some(session) = self.repository.get_active().await? {
            info!(session_id = %session.id, "Using existing active session");
            return Ok(session);
        }

        // Check for paused session to resume
        if let Some(mut session) = self.repository.get_ongoing().await? {
            info!(session_id = %session.id, "Resuming paused session");
            session.resume();
            self.repository.update(&session).await?;
            self.repository
                .save_event(&SessionEvent::resumed(session.id))
                .await?;
            return Ok(session);
        }

        // Create new session
        self.create(project_id, None, description).await
    }

    /// Pause a session
    pub async fn pause(&self, session_id: Uuid) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            return Err(Error::InvalidInput(format!(
                "Cannot pause session with status: {}",
                session.status
            )));
        }

        session.pause();
        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::paused(session_id))
            .await?;

        info!(session_id = %session_id, "Session paused");
        Ok(session)
    }

    /// Resume a session
    ///
    /// Pauses any other active session first.
    pub async fn resume(&self, session_id: Uuid) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            return Err(Error::InvalidInput(format!(
                "Cannot resume session with status: {}",
                session.status
            )));
        }

        // Pause any other active session
        if let Some(mut active) = self.repository.get_active().await? {
            if active.id != session_id {
                info!(session_id = %active.id, "Pausing existing active session");
                active.pause();
                self.repository.update(&active).await?;
                self.repository
                    .save_event(&SessionEvent::paused(active.id))
                    .await?;
            }
        }

        session.resume();
        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::resumed(session_id))
            .await?;

        info!(session_id = %session_id, "Session resumed");
        Ok(session)
    }

    /// Complete a session
    pub async fn complete(&self, session_id: Uuid) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            warn!(
                session_id = %session_id,
                status = %session.status,
                "Session already ended"
            );
            return Ok(session);
        }

        session.complete();
        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::completed(session_id))
            .await?;

        info!(session_id = %session_id, "Session completed");
        Ok(session)
    }

    /// Abandon a session
    pub async fn abandon(&self, session_id: Uuid) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            warn!(
                session_id = %session_id,
                status = %session.status,
                "Session already ended"
            );
            return Ok(session);
        }

        session.abandon();
        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::abandoned(session_id))
            .await?;

        info!(session_id = %session_id, "Session abandoned");
        Ok(session)
    }

    // ========== Context Switching ==========

    /// Switch the current project in a session
    pub async fn switch_project(&self, session_id: Uuid, project_id: Option<Uuid>) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            return Err(Error::InvalidInput(format!(
                "Cannot switch project in session with status: {}",
                session.status
            )));
        }

        let old_project_id = session.current_project_id;
        session.set_project(project_id);
        // Clear feature when switching projects
        session.set_feature(None);

        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::project_switched(
                session_id,
                old_project_id,
                project_id,
            ))
            .await?;

        info!(
            session_id = %session_id,
            old_project = ?old_project_id,
            new_project = ?project_id,
            "Switched project"
        );

        Ok(session)
    }

    /// Switch the current feature in a session
    pub async fn switch_feature(&self, session_id: Uuid, feature_id: Option<Uuid>) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            return Err(Error::InvalidInput(format!(
                "Cannot switch feature in session with status: {}",
                session.status
            )));
        }

        let old_feature_id = session.current_feature_id;
        session.set_feature(feature_id);

        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::feature_switched(
                session_id,
                old_feature_id,
                feature_id,
            ))
            .await?;

        info!(
            session_id = %session_id,
            old_feature = ?old_feature_id,
            new_feature = ?feature_id,
            "Switched feature"
        );

        Ok(session)
    }

    /// Update the session phase
    pub async fn set_phase(&self, session_id: Uuid, phase: SessionPhase) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        if session.status.has_ended() {
            return Err(Error::InvalidInput(format!(
                "Cannot update phase in session with status: {}",
                session.status
            )));
        }

        let old_phase = session.phase;
        session.set_phase(phase);

        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::phase_changed(
                session_id,
                old_phase.as_str(),
                phase.as_str(),
            ))
            .await?;

        info!(
            session_id = %session_id,
            old_phase = %old_phase,
            new_phase = %phase,
            "Changed phase"
        );

        Ok(session)
    }

    /// Record a checkpoint in the session
    pub async fn record_checkpoint(&self, session_id: Uuid, checkpoint_id: Uuid) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        session.set_checkpoint(checkpoint_id);
        self.repository.update(&session).await?;
        self.repository
            .save_event(&SessionEvent::checkpoint_created(session_id, checkpoint_id))
            .await?;

        info!(
            session_id = %session_id,
            checkpoint_id = %checkpoint_id,
            "Recorded checkpoint"
        );

        Ok(session)
    }

    /// Record an error in the session
    pub async fn record_error(
        &self,
        session_id: Uuid,
        error_message: &str,
        error_code: Option<&str>,
    ) -> Result<()> {
        self.repository
            .save_event(&SessionEvent::error(session_id, error_message, error_code))
            .await?;

        warn!(
            session_id = %session_id,
            error = %error_message,
            code = ?error_code,
            "Recorded error in session"
        );

        Ok(())
    }

    /// Touch a session to update last activity
    pub async fn touch(&self, session_id: Uuid) -> Result<Session> {
        let mut session = self
            .repository
            .get(session_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

        session.touch();
        self.repository.update(&session).await?;

        Ok(session)
    }

    // ========== Listing and Queries ==========

    /// List all sessions
    pub async fn list(&self, limit: Option<i32>) -> Result<Vec<SessionInfo>> {
        self.repository.list(limit).await
    }

    /// List sessions by status
    pub async fn list_by_status(&self, status: SessionStatus) -> Result<Vec<SessionInfo>> {
        self.repository.list_by_status(status).await
    }

    /// List sessions for a project
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<SessionInfo>> {
        self.repository.list_by_project(project_id).await
    }

    /// Get session events
    pub async fn get_events(&self, session_id: Uuid, limit: Option<i32>) -> Result<Vec<SessionEvent>> {
        self.repository.get_events(session_id, limit).await
    }

    // ========== Cleanup ==========

    /// Delete a session
    pub async fn delete(&self, session_id: Uuid) -> Result<bool> {
        self.repository.delete(session_id).await
    }

    /// Cleanup old sessions
    pub async fn cleanup_old_sessions(&self, days: i64) -> Result<u64> {
        let deleted = self.repository.delete_older_than(days).await?;
        if deleted > 0 {
            info!(deleted = deleted, days = days, "Cleaned up old sessions");
        }
        Ok(deleted)
    }

    /// Get session statistics
    pub async fn stats(&self) -> Result<SessionStats> {
        let active = self.repository.count_by_status(SessionStatus::Active).await?;
        let paused = self.repository.count_by_status(SessionStatus::Paused).await?;
        let completed = self.repository.count_by_status(SessionStatus::Completed).await?;
        let abandoned = self.repository.count_by_status(SessionStatus::Abandoned).await?;

        Ok(SessionStats {
            active,
            paused,
            completed,
            abandoned,
            total: active + paused + completed + abandoned,
        })
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    /// Number of active sessions
    pub active: i64,
    /// Number of paused sessions
    pub paused: i64,
    /// Number of completed sessions
    pub completed: i64,
    /// Number of abandoned sessions
    pub abandoned: i64,
    /// Total number of sessions
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    async fn create_test_manager() -> SessionManager {
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");
        SessionManager::new(db.pool().clone())
    }

    #[tokio::test]
    async fn test_create_session() {
        let manager = create_test_manager().await;

        let session = manager
            .create(None, None, Some("Test session".to_string()))
            .await
            .expect("Failed to create session");

        assert!(session.is_active());
        assert_eq!(session.description, Some("Test session".to_string()));
    }

    #[tokio::test]
    async fn test_create_pauses_existing() {
        let manager = create_test_manager().await;

        // Create first session
        let session1 = manager.create(None, None, Some("First".to_string())).await.unwrap();
        assert!(session1.is_active());

        // Create second session - should pause first
        let session2 = manager.create(None, None, Some("Second".to_string())).await.unwrap();
        assert!(session2.is_active());

        // First session should now be paused
        let session1 = manager.get(session1.id).await.unwrap().unwrap();
        assert!(session1.is_paused());
    }

    #[tokio::test]
    async fn test_get_or_create() {
        let manager = create_test_manager().await;

        // First call creates
        let session1 = manager.get_or_create(None, None).await.unwrap();

        // Second call returns same session
        let session2 = manager.get_or_create(None, None).await.unwrap();

        assert_eq!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_pause_and_resume() {
        let manager = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        assert!(session.is_active());

        // Pause
        let paused = manager.pause(session.id).await.unwrap();
        assert!(paused.is_paused());

        // Resume
        let resumed = manager.resume(session.id).await.unwrap();
        assert!(resumed.is_active());
    }

    #[tokio::test]
    async fn test_complete_session() {
        let manager = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();

        let completed = manager.complete(session.id).await.unwrap();
        assert_eq!(completed.status, SessionStatus::Completed);
        assert!(completed.status.has_ended());
    }

    #[tokio::test]
    async fn test_switch_project() {
        let manager = create_test_manager().await;

        // Create session without project
        let session = manager.create(None, None, None).await.unwrap();
        assert_eq!(session.current_project_id, None);

        // Switch to no project (clearing)
        let session = manager.switch_project(session.id, None).await.unwrap();
        assert_eq!(session.current_project_id, None);
    }

    #[tokio::test]
    async fn test_set_phase() {
        let manager = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        assert_eq!(session.phase, SessionPhase::Unknown);

        let session = manager.set_phase(session.id, SessionPhase::Building).await.unwrap();
        assert_eq!(session.phase, SessionPhase::Building);
    }

    #[tokio::test]
    async fn test_session_stats() {
        let manager = create_test_manager().await;

        // Create sessions in various states
        let s1 = manager.create(None, None, None).await.unwrap();
        manager.complete(s1.id).await.unwrap();

        let s2 = manager.create(None, None, None).await.unwrap();
        manager.pause(s2.id).await.unwrap();

        let _s3 = manager.create(None, None, None).await.unwrap();

        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.paused, 1);
        assert_eq!(stats.active, 1);
    }

    #[tokio::test]
    async fn test_session_events_recorded() {
        let manager = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        manager.pause(session.id).await.unwrap();
        manager.resume(session.id).await.unwrap();
        manager.complete(session.id).await.unwrap();

        let events = manager.get_events(session.id, None).await.unwrap();
        assert_eq!(events.len(), 4); // started, paused, resumed, completed
    }

    #[tokio::test]
    async fn test_cannot_modify_ended_session() {
        let manager = create_test_manager().await;

        let session = manager.create(None, None, None).await.unwrap();
        manager.complete(session.id).await.unwrap();

        // Should fail to pause completed session
        let result = manager.pause(session.id).await;
        assert!(result.is_err());

        // Should fail to switch project in completed session
        let result = manager.switch_project(session.id, None).await;
        assert!(result.is_err());
    }
}
