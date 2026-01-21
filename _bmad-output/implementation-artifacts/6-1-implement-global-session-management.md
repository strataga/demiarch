# Story 6.1: Implement Global Session Management

Status: ready-for-dev

## Story

As a user,
I want to have a global session that tracks active projects and context windows,
so that I can work across multiple projects without losing state.

## Acceptance Criteria

1. **Given** User opens Demiarch
   **When** Session starts
   **Then** Global session record is created in global_sessions table with status='active', started_at

2. **Given** Session is created
   **When** Session is initialized
   **Then** current_project_id is set to last used project or null

3. **Given** Session is tracking projects
   **When** Projects are opened
   **Then** active_project_ids JSON array tracks all open projects

4. **Given** Context windows are used
   **When** Context is tracked
   **Then** context_data JSON stores context windows for each project

5. **Given** User switches between projects
   **When** Project switch occurs
   **Then** current_project_id is updated to new project

6. **Given** Project switch occurs
   **When** Context windows are managed
   **Then** Context windows are preserved and restored for each project

7. **Given** Session exists
   **When** User restarts GUI/CLI
   **Then** Session persists in database across GUI/CLI restarts

## Tasks / Subtasks

- [ ] Task 1: Create global_sessions table schema (AC: #1, #2, #3, #4)
  - [ ] Add migration for global_sessions table
  - [ ] Include columns: id, status, started_at, completed_at, current_project_id, active_project_ids, context_data
  - [ ] Add indexes for status and started_at queries

- [ ] Task 2: Implement SessionManager service (AC: #1, #2)
  - [ ] Create SessionManager in demiarch-core
  - [ ] Implement start_session() method
  - [ ] Query last active project or default to null
  - [ ] Create session record in database

- [ ] Task 3: Implement project tracking (AC: #3, #5)
  - [ ] Track active_project_ids as JSON array
  - [ ] Implement add_project() and remove_project() methods
  - [ ] Update current_project_id on switch

- [ ] Task 4: Implement context window management (AC: #4, #6)
  - [ ] Serialize context windows to context_data JSON
  - [ ] Preserve context on project switch
  - [ ] Restore context when switching back

- [ ] Task 5: Implement session persistence (AC: #7)
  - [ ] Save session state on app close
  - [ ] Restore session on app start
  - [ ] Handle orphaned sessions (mark as 'interrupted')

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Global session tracking for multi-project support
- Context window preservation across project switches
- Session persistence across restarts

**Multi-Project Requirements:**
- Support for 3-5 concurrent projects
- Session tokens with short expiry, rotation on project switch
- Context data serialization/deserialization

### Session State Machine

```rust
pub enum SessionStatus {
    Active,      // Session in progress
    Completed,   // Clean shutdown
    Interrupted, // Unexpected termination
}

pub struct GlobalSession {
    pub id: String,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub current_project_id: Option<String>,
    pub active_project_ids: Vec<String>,
    pub context_data: serde_json::Value,
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/session/session.rs`
- `crates/demiarch-core/src/domain/session/manager.rs`
- `crates/demiarch-core/src/infrastructure/db/global_sessions.rs`

**Database Schema:**
```sql
CREATE TABLE global_sessions (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active',
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    current_project_id TEXT,
    active_project_ids TEXT NOT NULL DEFAULT '[]',  -- JSON array
    context_data TEXT NOT NULL DEFAULT '{}',  -- JSON object
    FOREIGN KEY (current_project_id) REFERENCES projects(id)
);

CREATE INDEX idx_global_sessions_status ON global_sessions(status);
CREATE INDEX idx_global_sessions_started ON global_sessions(started_at DESC);
```

### Testing Requirements

- Session creation tests
- Project tracking tests
- Context preservation tests
- Session persistence and recovery tests

### References

- [Source: docs/PRD.md#Multi-Project-Concurrency] - Multi-project requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Multi-Project] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
