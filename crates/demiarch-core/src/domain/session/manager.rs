//! Session manager for orchestrating session lifecycle
//!
//! Provides high-level operations for managing sessions, including
//! creation, lifecycle transitions, and context switching.

use super::event::SessionEvent;
use super::repository::SessionRepository;
use super::session::{
    RecoveryInfo, RecoveryResult, Session, SessionInfo, SessionPhase, SessionStatus,
};
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

    /// Recover a session after application restart
    ///
    /// This method should be called when the application starts to restore
    /// any ongoing session. It provides detailed recovery information including:
    /// - Whether the shutdown was clean (session was paused) or unclean (crash)
    /// - How long the session has been idle
    /// - Whether a checkpoint is available for code recovery
    ///
    /// # Recovery Behavior
    ///
    /// - If an active session exists (unclean shutdown), it's recovered and marked
    /// - If a paused session exists (clean shutdown), it's resumed
    /// - If no ongoing session exists, returns `NoneToRecover`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = manager.recover().await?;
    /// match result {
    ///     RecoveryResult::Recovered(info) => {
    ///         if info.was_unclean_shutdown {
    ///             warn!("Recovered from crash: {}", info.summary());
    ///         }
    ///         // Continue with recovered session
    ///     }
    ///     RecoveryResult::NoneToRecover => {
    ///         // No session to recover, can create new one
    ///     }
    ///     RecoveryResult::CreatedNew(session) => {
    ///         // A new session was created
    ///     }
    /// }
    /// ```
    pub async fn recover(&self) -> Result<RecoveryResult> {
        // Look for any ongoing session (active or paused)
        let Some(mut session) = self.repository.get_ongoing().await? else {
            info!("No session to recover");
            return Ok(RecoveryResult::NoneToRecover);
        };

        let previous_status = session.status;
        let was_unclean = previous_status == SessionStatus::Active;

        // If session was active (unclean shutdown), mark it as recovered
        // If session was paused (clean shutdown), just resume it
        session.resume();
        self.repository.update(&session).await?;

        // Record the appropriate event
        self.repository
            .save_event(&SessionEvent::recovered(
                session.id,
                previous_status.as_str(),
                was_unclean,
            ))
            .await?;

        let recovery_info = RecoveryInfo::new(session, previous_status);

        if was_unclean {
            warn!(
                session_id = %recovery_info.session.id,
                idle_mins = recovery_info.idle_duration.num_minutes(),
                "Recovered session after unclean shutdown"
            );
        } else {
            info!(
                session_id = %recovery_info.session.id,
                idle_mins = recovery_info.idle_duration.num_minutes(),
                "Recovered paused session"
            );
        }

        Ok(RecoveryResult::Recovered(recovery_info))
    }

    /// Recover a session or create a new one
    ///
    /// Convenience method that combines recovery and creation. Use this when
    /// you want to ensure there's always an active session after restart.
    ///
    /// # Arguments
    ///
    /// * `project_id` - Optional project ID for new sessions
    /// * `description` - Optional description for new sessions
    ///
    /// # Returns
    ///
    /// Returns `RecoveryResult::Recovered` if a session was recovered,
    /// or `RecoveryResult::CreatedNew` if a new session was created.
    pub async fn recover_or_create(
        &self,
        project_id: Option<Uuid>,
        description: Option<String>,
    ) -> Result<RecoveryResult> {
        match self.recover().await? {
            RecoveryResult::NoneToRecover => {
                let session = self.create(project_id, None, description).await?;
                Ok(RecoveryResult::CreatedNew(session))
            }
            result => Ok(result),
        }
    }

    /// Check if there's a recoverable session without recovering it
    ///
    /// Use this to check if recovery is needed before deciding how to proceed.
    pub async fn check_recoverable(&self) -> Result<Option<RecoveryInfo>> {
        let Some(session) = self.repository.get_ongoing().await? else {
            return Ok(None);
        };

        let previous_status = session.status;
        Ok(Some(RecoveryInfo::new(session, previous_status)))
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
    pub async fn switch_project(
        &self,
        session_id: Uuid,
        project_id: Option<Uuid>,
    ) -> Result<Session> {
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
    pub async fn switch_feature(
        &self,
        session_id: Uuid,
        feature_id: Option<Uuid>,
    ) -> Result<Session> {
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
    pub async fn record_checkpoint(
        &self,
        session_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<Session> {
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
    pub async fn get_events(
        &self,
        session_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<SessionEvent>> {
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

    /// Cleanup old session events
    ///
    /// Deletes events older than the specified number of days.
    pub async fn cleanup_old_events(&self, days: i64) -> Result<u64> {
        let deleted = self.repository.delete_old_events(days).await?;
        if deleted > 0 {
            info!(
                deleted = deleted,
                days = days,
                "Cleaned up old session events"
            );
        }
        Ok(deleted)
    }

    /// Cleanup events for ended sessions
    ///
    /// Deletes all events for sessions that are completed or abandoned.
    pub async fn cleanup_ended_session_events(&self) -> Result<u64> {
        let deleted = self.repository.delete_events_for_ended_sessions().await?;
        if deleted > 0 {
            info!(deleted = deleted, "Cleaned up events for ended sessions");
        }
        Ok(deleted)
    }

    /// Perform full cleanup of old data
    ///
    /// This method cleans up:
    /// - Sessions older than `session_days` that are completed or abandoned
    /// - Events older than `event_days` (default: same as session_days)
    ///
    /// Returns a summary of what was cleaned up.
    pub async fn full_cleanup(
        &self,
        session_days: i64,
        event_days: Option<i64>,
    ) -> Result<CleanupSummary> {
        let event_days = event_days.unwrap_or(session_days);

        let sessions_deleted = self.cleanup_old_sessions(session_days).await?;
        let events_deleted = self.cleanup_old_events(event_days).await?;

        let summary = CleanupSummary {
            sessions_deleted,
            events_deleted,
            session_days,
            event_days,
        };

        if summary.sessions_deleted > 0 || summary.events_deleted > 0 {
            info!(
                sessions = summary.sessions_deleted,
                events = summary.events_deleted,
                "Full cleanup completed"
            );
        }

        Ok(summary)
    }

    /// Get session statistics
    pub async fn stats(&self) -> Result<SessionStats> {
        let active = self
            .repository
            .count_by_status(SessionStatus::Active)
            .await?;
        let paused = self
            .repository
            .count_by_status(SessionStatus::Paused)
            .await?;
        let completed = self
            .repository
            .count_by_status(SessionStatus::Completed)
            .await?;
        let abandoned = self
            .repository
            .count_by_status(SessionStatus::Abandoned)
            .await?;

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

/// Summary of cleanup operations
#[derive(Debug, Clone)]
pub struct CleanupSummary {
    /// Number of sessions deleted
    pub sessions_deleted: u64,
    /// Number of events deleted
    pub events_deleted: u64,
    /// Age threshold for sessions (in days)
    pub session_days: i64,
    /// Age threshold for events (in days)
    pub event_days: i64,
}

impl CleanupSummary {
    /// Check if any cleanup was performed
    pub fn had_cleanup(&self) -> bool {
        self.sessions_deleted > 0 || self.events_deleted > 0
    }

    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        if !self.had_cleanup() {
            "No cleanup needed".to_string()
        } else {
            format!(
                "Cleaned up {} sessions (>{} days old) and {} events (>{} days old)",
                self.sessions_deleted, self.session_days, self.events_deleted, self.event_days
            )
        }
    }
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
        let session1 = manager
            .create(None, None, Some("First".to_string()))
            .await
            .unwrap();
        assert!(session1.is_active());

        // Create second session - should pause first
        let session2 = manager
            .create(None, None, Some("Second".to_string()))
            .await
            .unwrap();
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

        let session = manager
            .set_phase(session.id, SessionPhase::Building)
            .await
            .unwrap();
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

    // ========== Session Recovery Tests ==========

    #[tokio::test]
    async fn test_recover_no_session() {
        let manager = create_test_manager().await;

        // No session exists
        let result = manager.recover().await.unwrap();
        assert!(matches!(result, RecoveryResult::NoneToRecover));
    }

    #[tokio::test]
    async fn test_recover_paused_session_clean_shutdown() {
        let manager = create_test_manager().await;

        // Create and pause a session (simulates clean shutdown)
        let session = manager
            .create(None, None, Some("Test".to_string()))
            .await
            .unwrap();
        manager.pause(session.id).await.unwrap();

        // Recover should resume the paused session
        let result = manager.recover().await.unwrap();

        let RecoveryResult::Recovered(info) = result else {
            panic!("Expected RecoveryResult::Recovered, got {:?}", result);
        };
        assert_eq!(info.session.id, session.id);
        assert!(!info.was_unclean_shutdown);
        assert_eq!(info.previous_status, SessionStatus::Paused);
        assert!(info.session.is_active());
    }

    #[tokio::test]
    async fn test_recover_active_session_unclean_shutdown() {
        let manager = create_test_manager().await;

        // Create a session and leave it active (simulates unclean shutdown/crash)
        let session = manager
            .create(None, None, Some("Crashed".to_string()))
            .await
            .unwrap();
        assert!(session.is_active());

        // Recover should detect unclean shutdown
        let result = manager.recover().await.unwrap();

        let RecoveryResult::Recovered(info) = result else {
            panic!("Expected RecoveryResult::Recovered, got {:?}", result);
        };
        assert_eq!(info.session.id, session.id);
        assert!(info.was_unclean_shutdown);
        assert_eq!(info.previous_status, SessionStatus::Active);
        assert!(info.session.is_active());
    }

    #[tokio::test]
    async fn test_recover_records_event() {
        let manager = create_test_manager().await;

        // Create and pause a session
        let session = manager.create(None, None, None).await.unwrap();
        manager.pause(session.id).await.unwrap();

        // Recover
        manager.recover().await.unwrap();

        // Check that recovered event was recorded
        let events = manager.get_events(session.id, None).await.unwrap();
        let recovered_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == super::super::event::SessionEventType::Recovered)
            .collect();

        assert_eq!(recovered_events.len(), 1);
        let data = recovered_events[0].data.as_ref().unwrap();
        assert_eq!(data["previous_status"], "paused");
        assert_eq!(data["was_unclean_shutdown"], false);
    }

    #[tokio::test]
    async fn test_recover_or_create_with_existing() {
        let manager = create_test_manager().await;

        // Create and pause a session
        let session = manager.create(None, None, None).await.unwrap();
        manager.pause(session.id).await.unwrap();

        // Should recover existing session
        let result = manager.recover_or_create(None, None).await.unwrap();

        match result {
            RecoveryResult::Recovered(info) => {
                assert_eq!(info.session.id, session.id);
            }
            _ => panic!("Expected RecoveryResult::Recovered"),
        }
    }

    #[tokio::test]
    async fn test_recover_or_create_new() {
        let manager = create_test_manager().await;

        // No existing session - should create new
        let result = manager
            .recover_or_create(None, Some("New".to_string()))
            .await
            .unwrap();

        match result {
            RecoveryResult::CreatedNew(session) => {
                assert!(session.is_active());
                assert_eq!(session.description, Some("New".to_string()));
            }
            _ => panic!("Expected RecoveryResult::CreatedNew"),
        }
    }

    #[tokio::test]
    async fn test_check_recoverable_exists() {
        let manager = create_test_manager().await;

        // Create a session
        let session = manager.create(None, None, None).await.unwrap();
        manager.pause(session.id).await.unwrap();

        // Should find recoverable session
        let info = manager.check_recoverable().await.unwrap();
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.session.id, session.id);
    }

    #[tokio::test]
    async fn test_check_recoverable_none() {
        let manager = create_test_manager().await;

        // No session - should return None
        let info = manager.check_recoverable().await.unwrap();
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_check_recoverable_completed_not_recoverable() {
        let manager = create_test_manager().await;

        // Create and complete a session
        let session = manager.create(None, None, None).await.unwrap();
        manager.complete(session.id).await.unwrap();

        // Completed sessions are not recoverable
        let info = manager.check_recoverable().await.unwrap();
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_recovery_info_summary() {
        let manager = create_test_manager().await;

        // Create and leave active (unclean shutdown)
        let _session = manager.create(None, None, None).await.unwrap();

        let result = manager.recover().await.unwrap();

        if let RecoveryResult::Recovered(info) = result {
            let summary = info.summary();
            assert!(summary.contains("unclean shutdown"));
            assert!(summary.contains(&info.session.id.to_string()));
        } else {
            panic!("Expected RecoveryResult::Recovered");
        }
    }

    #[tokio::test]
    async fn test_recovery_with_checkpoint() {
        let manager = create_test_manager().await;

        // Create session - we can't actually record a checkpoint without a valid
        // checkpoint in the checkpoints table (foreign key constraint), so we test
        // by creating a session and checking recovery works with checkpoint_id = None
        let session = manager.create(None, None, None).await.unwrap();
        manager.pause(session.id).await.unwrap();

        // Recovery info should work correctly (has_checkpoint = false since no checkpoint)
        let result = manager.recover().await.unwrap();

        if let RecoveryResult::Recovered(info) = result {
            assert!(!info.has_checkpoint);
            assert_eq!(info.session.last_checkpoint_id, None);
        } else {
            panic!("Expected RecoveryResult::Recovered");
        }
    }

    #[tokio::test]
    async fn test_recovery_has_checkpoint_detection() {
        // Test the RecoveryInfo has_checkpoint field logic directly
        let mut session = super::super::session::Session::new(None, None, None);

        // Without checkpoint
        let info = super::super::session::RecoveryInfo::new(session.clone(), SessionStatus::Paused);
        assert!(!info.has_checkpoint);

        // With checkpoint
        session.last_checkpoint_id = Some(Uuid::new_v4());
        let info = super::super::session::RecoveryInfo::new(session, SessionStatus::Paused);
        assert!(info.has_checkpoint);
    }
}
