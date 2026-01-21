# Story 6.4: Session Recovery on Restart

Status: ready-for-dev

## Story

As a user,
I want my session state to be restored automatically when I restart Demiarch,
so that I can continue working without setup.

## Acceptance Criteria

1. **Given** User had an active session with projects and context windows
   **When** User reopens Demiarch after restart
   **Then** System queries global_sessions table for last active session

2. **Given** Last session is found
   **When** Session state is reconstructed
   **Then** Session state is reconstructed from context_data JSON

3. **Given** Session state is restored
   **When** Current project is set
   **Then** current_project_id is restored

4. **Given** Session is fully restored
   **When** Projects are loaded
   **Then** All active projects are loaded into memory

5. **Given** GUI is displayed
   **When** Projects are shown
   **Then** GUI displays projects in their last viewed state

6. **Given** Recovery completes
   **When** User is notified
   **Then** User sees: "Welcome back! Restored session with 3 active projects"

## Tasks / Subtasks

- [ ] Task 1: Implement session query on startup (AC: #1)
  - [ ] Query global_sessions for last active or interrupted session
  - [ ] Order by started_at DESC, limit 1
  - [ ] Handle case where no session exists

- [ ] Task 2: Implement state reconstruction (AC: #2, #3)
  - [ ] Deserialize context_data JSON
  - [ ] Restore current_project_id
  - [ ] Validate project still exists

- [ ] Task 3: Load active projects (AC: #4)
  - [ ] Load each project from active_project_ids
  - [ ] Handle missing projects gracefully
  - [ ] Initialize project state from database

- [ ] Task 4: Restore GUI state (AC: #5)
  - [ ] Restore last viewed project
  - [ ] Restore UI state (active panel, scroll position)
  - [ ] Restore context windows

- [ ] Task 5: Display recovery notification (AC: #6)
  - [ ] Show toast notification with project count
  - [ ] List restored projects if applicable
  - [ ] Handle partial recovery gracefully

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Session persistence and recovery
- Graceful handling of missing data
- State serialization/deserialization

**Recovery Requirements:**
- Handle corrupted session data
- Handle deleted projects
- Maintain user experience continuity

### Recovery Flow

```rust
pub async fn recover_session(db: &Database) -> Result<Option<GlobalSession>> {
    // 1. Find last session
    let last_session = db.query_one::<GlobalSession>(
        "SELECT * FROM global_sessions
         WHERE status IN ('active', 'interrupted')
         ORDER BY started_at DESC LIMIT 1"
    ).await?;

    if let Some(mut session) = last_session {
        // 2. Mark previous session as interrupted if still active
        if session.status == SessionStatus::Active {
            session.status = SessionStatus::Interrupted;
            db.update(&session).await?;
        }

        // 3. Create new session with recovered state
        let new_session = GlobalSession {
            id: Uuid::new_v4().to_string(),
            status: SessionStatus::Active,
            started_at: Utc::now(),
            completed_at: None,
            current_project_id: session.current_project_id,
            active_project_ids: session.active_project_ids,
            context_data: session.context_data,
        };

        // 4. Validate and load projects
        let valid_projects = validate_projects(&new_session.active_project_ids, db).await?;
        new_session.active_project_ids = valid_projects;

        db.insert(&new_session).await?;
        Ok(Some(new_session))
    } else {
        Ok(None)
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/session/recovery.rs`
- `crates/demiarch-core/src/application/startup.rs`
- `crates/demiarch-gui/src/hooks/useSessionRecovery.ts`

**Database Queries:**
```sql
-- Find last session for recovery
SELECT * FROM global_sessions
WHERE status IN ('active', 'interrupted')
ORDER BY started_at DESC
LIMIT 1;

-- Validate project exists
SELECT id FROM projects WHERE id IN (?, ?, ?);
```

### Testing Requirements

- Session recovery tests
- Corrupted data handling tests
- Missing project handling tests
- Partial recovery tests

### References

- [Source: docs/PRD.md#Multi-Project-Concurrency] - Session requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Multi-Project] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
