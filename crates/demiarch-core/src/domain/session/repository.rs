//! Session repository for database operations
//!
//! Handles all database interactions for sessions and session events.

use super::event::{SessionEvent, SessionEventType};
use super::session::{Session, SessionInfo, SessionPhase, SessionStatus};
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Repository for session database operations
#[derive(Debug, Clone)]
pub struct SessionRepository {
    pool: SqlitePool,
}

impl SessionRepository {
    /// Create a new repository with the given connection pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ========== Session CRUD ==========

    /// Save a new session to the database
    pub async fn save(&self, session: &Session) -> Result<()> {
        let id = session.id.to_string();
        let current_project_id = session.current_project_id.map(|p| p.to_string());
        let current_feature_id = session.current_feature_id.map(|f| f.to_string());
        let last_checkpoint_id = session.last_checkpoint_id.map(|c| c.to_string());
        let metadata = session.metadata.as_ref().map(|m| m.to_string());

        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, created_at, updated_at, last_activity,
                current_project_id, current_feature_id,
                status, phase, description,
                last_checkpoint_id, metadata
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(session.created_at)
        .bind(session.updated_at)
        .bind(session.last_activity)
        .bind(&current_project_id)
        .bind(&current_feature_id)
        .bind(session.status.as_str())
        .bind(session.phase.as_str())
        .bind(&session.description)
        .bind(&last_checkpoint_id)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    /// Update an existing session
    pub async fn update(&self, session: &Session) -> Result<()> {
        let id = session.id.to_string();
        let current_project_id = session.current_project_id.map(|p| p.to_string());
        let current_feature_id = session.current_feature_id.map(|f| f.to_string());
        let last_checkpoint_id = session.last_checkpoint_id.map(|c| c.to_string());
        let metadata = session.metadata.as_ref().map(|m| m.to_string());

        sqlx::query(
            r#"
            UPDATE sessions SET
                updated_at = ?,
                last_activity = ?,
                current_project_id = ?,
                current_feature_id = ?,
                status = ?,
                phase = ?,
                description = ?,
                last_checkpoint_id = ?,
                metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(session.updated_at)
        .bind(session.last_activity)
        .bind(&current_project_id)
        .bind(&current_feature_id)
        .bind(session.status.as_str())
        .bind(session.phase.as_str())
        .bind(&session.description)
        .bind(&last_checkpoint_id)
        .bind(&metadata)
        .bind(&id)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    /// Get a session by ID
    pub async fn get(&self, session_id: Uuid) -> Result<Option<Session>> {
        let id = session_id.to_string();

        let row: Option<SessionRow> = sqlx::query_as(
            r#"
            SELECT id, created_at, updated_at, last_activity,
                   current_project_id, current_feature_id,
                   status, phase, description,
                   last_checkpoint_id, metadata
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        match row {
            Some(row) => Ok(Some(row.into_session()?)),
            None => Ok(None),
        }
    }

    /// Get the most recent active session
    pub async fn get_active(&self) -> Result<Option<Session>> {
        let row: Option<SessionRow> = sqlx::query_as(
            r#"
            SELECT id, created_at, updated_at, last_activity,
                   current_project_id, current_feature_id,
                   status, phase, description,
                   last_checkpoint_id, metadata
            FROM sessions
            WHERE status = 'active'
            ORDER BY last_activity DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        match row {
            Some(row) => Ok(Some(row.into_session()?)),
            None => Ok(None),
        }
    }

    /// Get the most recent session (active or paused)
    pub async fn get_ongoing(&self) -> Result<Option<Session>> {
        let row: Option<SessionRow> = sqlx::query_as(
            r#"
            SELECT id, created_at, updated_at, last_activity,
                   current_project_id, current_feature_id,
                   status, phase, description,
                   last_checkpoint_id, metadata
            FROM sessions
            WHERE status IN ('active', 'paused')
            ORDER BY last_activity DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        match row {
            Some(row) => Ok(Some(row.into_session()?)),
            None => Ok(None),
        }
    }

    /// List all sessions, ordered by last_activity DESC
    pub async fn list(&self, limit: Option<i32>) -> Result<Vec<SessionInfo>> {
        let limit = limit.unwrap_or(50);

        let rows: Vec<SessionInfoRow> = sqlx::query_as(
            r#"
            SELECT id, status, phase, current_project_id, description, created_at, last_activity
            FROM sessions
            ORDER BY last_activity DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter().map(|row| row.into_session_info()).collect()
    }

    /// List sessions by status
    pub async fn list_by_status(&self, status: SessionStatus) -> Result<Vec<SessionInfo>> {
        let rows: Vec<SessionInfoRow> = sqlx::query_as(
            r#"
            SELECT id, status, phase, current_project_id, description, created_at, last_activity
            FROM sessions
            WHERE status = ?
            ORDER BY last_activity DESC
            "#,
        )
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter().map(|row| row.into_session_info()).collect()
    }

    /// List sessions for a specific project
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<SessionInfo>> {
        let project_id_str = project_id.to_string();

        let rows: Vec<SessionInfoRow> = sqlx::query_as(
            r#"
            SELECT id, status, phase, current_project_id, description, created_at, last_activity
            FROM sessions
            WHERE current_project_id = ?
            ORDER BY last_activity DESC
            "#,
        )
        .bind(&project_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter().map(|row| row.into_session_info()).collect()
    }

    /// Delete a session by ID
    pub async fn delete(&self, session_id: Uuid) -> Result<bool> {
        let id = session_id.to_string();

        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete old completed/abandoned sessions
    pub async fn delete_older_than(&self, days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE status IN ('completed', 'abandoned')
            AND last_activity < ?
            "#,
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// Count sessions by status
    pub async fn count_by_status(&self, status: SessionStatus) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sessions WHERE status = ?
            "#,
        )
        .bind(status.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(count)
    }

    // ========== Session Events ==========

    /// Save a session event
    pub async fn save_event(&self, event: &SessionEvent) -> Result<()> {
        let id = event.id.to_string();
        let session_id = event.session_id.to_string();
        let data = event.data.as_ref().map(|d| d.to_string());

        sqlx::query(
            r#"
            INSERT INTO session_events (id, session_id, event_type, data, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&session_id)
        .bind(event.event_type.as_str())
        .bind(&data)
        .bind(event.created_at)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    /// Get events for a session
    pub async fn get_events(&self, session_id: Uuid, limit: Option<i32>) -> Result<Vec<SessionEvent>> {
        let session_id_str = session_id.to_string();
        let limit = limit.unwrap_or(100);

        let rows: Vec<SessionEventRow> = sqlx::query_as(
            r#"
            SELECT id, session_id, event_type, data, created_at
            FROM session_events
            WHERE session_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&session_id_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter().map(|row| row.into_event()).collect()
    }

    /// Get events of a specific type for a session
    pub async fn get_events_by_type(
        &self,
        session_id: Uuid,
        event_type: SessionEventType,
    ) -> Result<Vec<SessionEvent>> {
        let session_id_str = session_id.to_string();

        let rows: Vec<SessionEventRow> = sqlx::query_as(
            r#"
            SELECT id, session_id, event_type, data, created_at
            FROM session_events
            WHERE session_id = ? AND event_type = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(&session_id_str)
        .bind(event_type.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        rows.into_iter().map(|row| row.into_event()).collect()
    }

    /// Count events for a session
    pub async fn count_events(&self, session_id: Uuid) -> Result<i64> {
        let session_id_str = session_id.to_string();

        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM session_events WHERE session_id = ?
            "#,
        )
        .bind(&session_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(count)
    }

    /// Delete old session events across all sessions
    ///
    /// Deletes events older than the specified number of days.
    /// Returns the number of events deleted.
    pub async fn delete_old_events(&self, days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM session_events
            WHERE created_at < ?
            "#,
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// Delete events for sessions that have ended (completed or abandoned)
    ///
    /// Useful for cleaning up event history for terminated sessions.
    /// Returns the number of events deleted.
    pub async fn delete_events_for_ended_sessions(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM session_events
            WHERE session_id IN (
                SELECT id FROM sessions
                WHERE status IN ('completed', 'abandoned')
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// Delete events for a specific session
    pub async fn delete_events_for_session(&self, session_id: Uuid) -> Result<u64> {
        let session_id_str = session_id.to_string();

        let result = sqlx::query(
            r#"
            DELETE FROM session_events
            WHERE session_id = ?
            "#,
        )
        .bind(&session_id_str)
        .execute(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(result.rows_affected())
    }

    /// Count total events across all sessions
    pub async fn count_all_events(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM session_events
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::DatabaseError)?;

        Ok(count)
    }
}

// ========== Database Row Types ==========

/// Database row for full session
#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    current_project_id: Option<String>,
    current_feature_id: Option<String>,
    status: String,
    phase: String,
    description: Option<String>,
    last_checkpoint_id: Option<String>,
    metadata: Option<String>,
}

impl SessionRow {
    fn into_session(self) -> Result<Session> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| Error::Parse(format!("Invalid session ID: {}", e)))?;
        let current_project_id = self
            .current_project_id
            .map(|p| Uuid::parse_str(&p))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;
        let current_feature_id = self
            .current_feature_id
            .map(|f| Uuid::parse_str(&f))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid feature ID: {}", e)))?;
        let last_checkpoint_id = self
            .last_checkpoint_id
            .map(|c| Uuid::parse_str(&c))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid checkpoint ID: {}", e)))?;
        let status = SessionStatus::from_str(&self.status)
            .ok_or_else(|| Error::Parse(format!("Invalid session status: {}", self.status)))?;
        let phase = SessionPhase::from_str(&self.phase)
            .ok_or_else(|| Error::Parse(format!("Invalid session phase: {}", self.phase)))?;
        let metadata = self
            .metadata
            .map(|m| serde_json::from_str(&m))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid metadata JSON: {}", e)))?;

        Ok(Session {
            id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_activity: self.last_activity,
            current_project_id,
            current_feature_id,
            status,
            phase,
            description: self.description,
            last_checkpoint_id,
            metadata,
        })
    }
}

/// Database row for session info (lightweight)
#[derive(sqlx::FromRow)]
struct SessionInfoRow {
    id: String,
    status: String,
    phase: String,
    current_project_id: Option<String>,
    description: Option<String>,
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
}

impl SessionInfoRow {
    fn into_session_info(self) -> Result<SessionInfo> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| Error::Parse(format!("Invalid session ID: {}", e)))?;
        let current_project_id = self
            .current_project_id
            .map(|p| Uuid::parse_str(&p))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid project ID: {}", e)))?;
        let status = SessionStatus::from_str(&self.status)
            .ok_or_else(|| Error::Parse(format!("Invalid session status: {}", self.status)))?;
        let phase = SessionPhase::from_str(&self.phase)
            .ok_or_else(|| Error::Parse(format!("Invalid session phase: {}", self.phase)))?;

        Ok(SessionInfo {
            id,
            status,
            phase,
            current_project_id,
            description: self.description,
            created_at: self.created_at,
            last_activity: self.last_activity,
        })
    }
}

/// Database row for session event
#[derive(sqlx::FromRow)]
struct SessionEventRow {
    id: String,
    session_id: String,
    event_type: String,
    data: Option<String>,
    created_at: DateTime<Utc>,
}

impl SessionEventRow {
    fn into_event(self) -> Result<SessionEvent> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| Error::Parse(format!("Invalid event ID: {}", e)))?;
        let session_id = Uuid::parse_str(&self.session_id)
            .map_err(|e| Error::Parse(format!("Invalid session ID: {}", e)))?;
        let event_type = SessionEventType::from_str(&self.event_type)
            .ok_or_else(|| Error::Parse(format!("Invalid event type: {}", self.event_type)))?;
        let data = self
            .data
            .map(|d| serde_json::from_str(&d))
            .transpose()
            .map_err(|e| Error::Parse(format!("Invalid event data JSON: {}", e)))?;

        Ok(SessionEvent {
            id,
            session_id,
            event_type,
            data,
            created_at: self.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    async fn create_test_db() -> SqlitePool {
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");
        db.pool().clone()
    }

    #[tokio::test]
    async fn test_save_and_get_session() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        let session = Session::new(None, None, Some("Test session".to_string()));

        // Save
        repo.save(&session).await.expect("Failed to save");

        // Get
        let retrieved = repo
            .get(session.id)
            .await
            .expect("Failed to get")
            .expect("Session not found");

        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.description, session.description);
        assert_eq!(retrieved.status, SessionStatus::Active);
    }

    #[tokio::test]
    async fn test_update_session() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        let mut session = Session::new(None, None, Some("Test".to_string()));
        repo.save(&session).await.expect("Failed to save");

        // Update
        session.pause();
        session.description = Some("Updated".to_string());
        repo.update(&session).await.expect("Failed to update");

        // Verify
        let retrieved = repo.get(session.id).await.expect("Failed to get").unwrap();
        assert_eq!(retrieved.status, SessionStatus::Paused);
        assert_eq!(retrieved.description, Some("Updated".to_string()));
    }

    #[tokio::test]
    async fn test_get_active_session() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        // No active session
        let active = repo.get_active().await.expect("Failed to get active");
        assert!(active.is_none());

        // Create active session
        let session = Session::new(None, None, None);
        repo.save(&session).await.expect("Failed to save");

        // Should find active session
        let active = repo.get_active().await.expect("Failed to get active");
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        // Create multiple sessions
        for i in 0..3 {
            let session = Session::new(None, None, Some(format!("Session {}", i)));
            repo.save(&session).await.expect("Failed to save");
        }

        let sessions = repo.list(None).await.expect("Failed to list");
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        let session = Session::new(None, None, None);
        repo.save(&session).await.expect("Failed to save");

        // Delete
        let deleted = repo.delete(session.id).await.expect("Failed to delete");
        assert!(deleted);

        // Should not exist
        let retrieved = repo.get(session.id).await.expect("Failed to get");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_session_events() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        let session = Session::new(None, None, None);
        repo.save(&session).await.expect("Failed to save session");

        // Create events
        let event1 = SessionEvent::started(session.id);
        let event2 = SessionEvent::paused(session.id);
        repo.save_event(&event1).await.expect("Failed to save event");
        repo.save_event(&event2).await.expect("Failed to save event");

        // Get events
        let events = repo.get_events(session.id, None).await.expect("Failed to get events");
        assert_eq!(events.len(), 2);

        // Count events
        let count = repo.count_events(session.id).await.expect("Failed to count");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_events_by_type() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        let session = Session::new(None, None, None);
        repo.save(&session).await.expect("Failed to save session");

        // Create events
        repo.save_event(&SessionEvent::started(session.id)).await.unwrap();
        repo.save_event(&SessionEvent::paused(session.id)).await.unwrap();
        repo.save_event(&SessionEvent::resumed(session.id)).await.unwrap();
        repo.save_event(&SessionEvent::paused(session.id)).await.unwrap();

        // Get only paused events
        let paused_events = repo
            .get_events_by_type(session.id, SessionEventType::Paused)
            .await
            .expect("Failed to get events");
        assert_eq!(paused_events.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_events_for_session() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        let session = Session::new(None, None, None);
        repo.save(&session).await.expect("Failed to save session");

        // Create events
        repo.save_event(&SessionEvent::started(session.id)).await.unwrap();
        repo.save_event(&SessionEvent::paused(session.id)).await.unwrap();
        repo.save_event(&SessionEvent::resumed(session.id)).await.unwrap();

        // Verify events exist
        let count = repo.count_events(session.id).await.unwrap();
        assert_eq!(count, 3);

        // Delete events
        let deleted = repo.delete_events_for_session(session.id).await.unwrap();
        assert_eq!(deleted, 3);

        // Verify events are gone
        let count = repo.count_events(session.id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_delete_events_for_ended_sessions() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        // Create an active session
        let mut active_session = Session::new(None, None, Some("Active".to_string()));
        repo.save(&active_session).await.unwrap();
        repo.save_event(&SessionEvent::started(active_session.id)).await.unwrap();

        // Create a completed session
        let mut completed_session = Session::new(None, None, Some("Completed".to_string()));
        completed_session.complete();
        repo.save(&completed_session).await.unwrap();
        repo.save_event(&SessionEvent::started(completed_session.id)).await.unwrap();
        repo.save_event(&SessionEvent::completed(completed_session.id)).await.unwrap();

        // Verify total event counts
        let active_count = repo.count_events(active_session.id).await.unwrap();
        let completed_count = repo.count_events(completed_session.id).await.unwrap();
        assert_eq!(active_count, 1);
        assert_eq!(completed_count, 2);

        // Delete events for ended sessions
        let deleted = repo.delete_events_for_ended_sessions().await.unwrap();
        assert_eq!(deleted, 2); // Should delete events for completed session only

        // Verify active session events still exist
        let active_count = repo.count_events(active_session.id).await.unwrap();
        assert_eq!(active_count, 1);

        // Verify completed session events are gone
        let completed_count = repo.count_events(completed_session.id).await.unwrap();
        assert_eq!(completed_count, 0);
    }

    #[tokio::test]
    async fn test_count_all_events() {
        let pool = create_test_db().await;
        let repo = SessionRepository::new(pool);

        // Create two sessions with events
        let session1 = Session::new(None, None, Some("Session 1".to_string()));
        let session2 = Session::new(None, None, Some("Session 2".to_string()));
        repo.save(&session1).await.unwrap();
        repo.save(&session2).await.unwrap();

        repo.save_event(&SessionEvent::started(session1.id)).await.unwrap();
        repo.save_event(&SessionEvent::paused(session1.id)).await.unwrap();
        repo.save_event(&SessionEvent::started(session2.id)).await.unwrap();

        // Count all events
        let count = repo.count_all_events().await.unwrap();
        assert_eq!(count, 3);
    }
}
