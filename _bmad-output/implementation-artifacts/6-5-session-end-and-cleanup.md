# Story 6.5: Session End and Cleanup

Status: ready-for-dev

## Story

As a user,
I want to explicitly end my session with cleanup,
so that resources are released and state is saved properly.

## Acceptance Criteria

1. **Given** User is in an active session
   **When** User closes Demiarch or clicks "End Session"
   **Then** global_sessions.status is updated to 'completed'

2. **Given** Session status is updated
   **When** Timestamp is recorded
   **Then** completed_at is set to current time

3. **Given** Session is ending
   **When** Resources are released
   **Then** All resource locks for session are released

4. **Given** Resources are released
   **When** Context is saved
   **Then** Context windows are serialized to context_data before cleanup

5. **Given** Cleanup is complete
   **When** Audit is recorded
   **Then** Audit log entry is created: event_type='session_end', details={session_id}

6. **Given** Cleanup runs
   **When** Performance is measured
   **Then** Session cleanup completes within 2 seconds

## Tasks / Subtasks

- [ ] Task 1: Implement session end handler (AC: #1, #2)
  - [ ] Add end_session() method to SessionManager
  - [ ] Update status to 'completed'
  - [ ] Set completed_at timestamp

- [ ] Task 2: Implement resource cleanup (AC: #3)
  - [ ] Query all active locks for session
  - [ ] Release each lock
  - [ ] Handle orphaned locks gracefully

- [ ] Task 3: Implement context serialization (AC: #4)
  - [ ] Serialize current context windows
  - [ ] Save to context_data JSON
  - [ ] Preserve for recovery

- [ ] Task 4: Implement audit logging (AC: #5)
  - [ ] Create audit_log entry for session_end
  - [ ] Include session_id in details
  - [ ] Record cleanup metrics

- [ ] Task 5: Implement cleanup performance (AC: #6)
  - [ ] Optimize cleanup operations
  - [ ] Use transaction for atomicity
  - [ ] Add timeout handling
  - [ ] Measure and log cleanup duration

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Clean session termination
- Resource release on shutdown
- Audit trail for session events

**Performance Requirements:**
- Cleanup within 2 seconds
- Async resource release
- Transaction-based state updates

### Session End Flow

```rust
pub async fn end_session(&self, session_id: &str) -> Result<()> {
    let start = Instant::now();

    // Use transaction for atomicity
    let mut tx = self.db.begin().await?;

    // 1. Serialize context before cleanup
    let context_data = self.serialize_context_windows().await?;

    // 2. Update session status
    sqlx::query!(
        "UPDATE global_sessions
         SET status = 'completed',
             completed_at = CURRENT_TIMESTAMP,
             context_data = ?
         WHERE id = ?",
        context_data,
        session_id
    )
    .execute(&mut tx)
    .await?;

    // 3. Release all locks for this session
    sqlx::query!(
        "UPDATE resource_locks
         SET released_at = CURRENT_TIMESTAMP
         WHERE released_at IS NULL
         AND agent_id IN (SELECT id FROM agent_executions WHERE session_id = ?)",
        session_id
    )
    .execute(&mut tx)
    .await?;

    // 4. Create audit log entry
    sqlx::query!(
        "INSERT INTO audit_log (id, event_type, event_time, details)
         VALUES (?, 'session_end', CURRENT_TIMESTAMP, ?)",
        Uuid::new_v4().to_string(),
        json!({ "session_id": session_id, "cleanup_ms": start.elapsed().as_millis() })
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    let duration = start.elapsed();
    if duration.as_secs() > 2 {
        warn!("Session cleanup took {}ms, exceeding 2s target", duration.as_millis());
    }

    Ok(())
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/session/cleanup.rs`
- `crates/demiarch-core/src/application/shutdown.rs`
- `crates/demiarch-gui/src/hooks/useSessionEnd.ts`

**Database Operations:**
```sql
-- Update session status
UPDATE global_sessions
SET status = 'completed',
    completed_at = CURRENT_TIMESTAMP,
    context_data = ?
WHERE id = ?;

-- Release session locks
UPDATE resource_locks
SET released_at = CURRENT_TIMESTAMP
WHERE released_at IS NULL
AND agent_id IN (
    SELECT id FROM agent_executions WHERE session_id = ?
);

-- Audit log entry
INSERT INTO audit_log (id, event_type, event_time, details)
VALUES (?, 'session_end', CURRENT_TIMESTAMP, ?);
```

### Testing Requirements

- Session end tests
- Resource release tests
- Context serialization tests
- Performance tests (< 2 seconds)
- Audit logging tests

### References

- [Source: docs/PRD.md#Multi-Project-Concurrency] - Session requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Multi-Project] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
