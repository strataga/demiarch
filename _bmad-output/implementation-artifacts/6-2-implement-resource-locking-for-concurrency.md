# Story 6.2: Implement Resource Locking for Concurrency

Status: ready-for-dev

## Story

As a user with multiple projects,
I want agents to acquire locks on resources (files, database, LLM, features) to prevent conflicts,
so that concurrent operations don't corrupt data.

## Acceptance Criteria

1. **Given** Multiple agents are running across projects
   **When** Agent needs to access a resource (e.g., write to file)
   **Then** Agent calls LockManager::acquire with resource_key (project_id, resource_type, resource_key)

2. **Given** Lock is requested
   **When** Resource is free or timeout occurs
   **Then** Lock is granted if resource is free or timeout occurs after 300 seconds

3. **Given** Lock is granted
   **When** Lock is recorded
   **Then** Lock is recorded in resource_locks table with agent_id, acquired_at

4. **Given** Lock is held by another agent
   **When** Request is made
   **Then** Request blocks until timeout

5. **Given** Agent completes operation
   **When** Lock is released
   **Then** Lock is released via LockGuard::release or Drop implementation

6. **Given** Lock is released
   **When** Database is updated
   **Then** resource_locks.released_at is set to current time

7. **Given** Deadlock is detected
   **When** Force release occurs
   **Then** Lock is force-released after timeout and warning is logged: "Deadlock detected on resource {resource_key}"

## Tasks / Subtasks

- [ ] Task 1: Create resource_locks table schema (AC: #3, #6)
  - [ ] Add migration for resource_locks table
  - [ ] Include columns: id, project_id, resource_type, resource_key, agent_id, acquired_at, released_at, timeout_at
  - [ ] Add composite index for resource lookups

- [ ] Task 2: Implement LockManager service (AC: #1, #2)
  - [ ] Create LockManager with async semaphores
  - [ ] Implement acquire() method with timeout
  - [ ] Support resource types: file, database, llm, feature
  - [ ] Default timeout: 300 seconds

- [ ] Task 3: Implement LockGuard RAII pattern (AC: #5, #6)
  - [ ] Create LockGuard struct with Drop implementation
  - [ ] Automatically release lock on drop
  - [ ] Update released_at in database

- [ ] Task 4: Implement blocking behavior (AC: #4)
  - [ ] Queue lock requests
  - [ ] Block until lock is available or timeout
  - [ ] Return error on timeout

- [ ] Task 5: Implement deadlock detection (AC: #7)
  - [ ] Monitor lock duration
  - [ ] Detect locks held beyond timeout
  - [ ] Force-release with warning log
  - [ ] Add audit_log entry for deadlock events

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Async semaphores with timeouts for resource locking
- RAII pattern for automatic lock release
- Deadlock detection and forced release

**Multi-Project Requirements:**
- Locks with timeouts + forced release on deadlock
- Resource isolation per project

### Lock Types

```rust
pub enum ResourceType {
    File,
    Database,
    Llm,
    Feature,
}

pub struct ResourceLock {
    pub id: String,
    pub project_id: String,
    pub resource_type: ResourceType,
    pub resource_key: String,
    pub agent_id: Option<String>,
    pub acquired_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub timeout_at: DateTime<Utc>,
}

pub struct LockGuard {
    lock_id: String,
    lock_manager: Arc<LockManager>,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Release lock on drop
        self.lock_manager.release(&self.lock_id);
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/concurrency/lock_manager.rs`
- `crates/demiarch-core/src/domain/concurrency/lock_guard.rs`
- `crates/demiarch-core/src/infrastructure/db/resource_locks.rs`

**Database Schema:**
```sql
CREATE TABLE resource_locks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_key TEXT NOT NULL,
    agent_id TEXT,
    acquired_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    released_at DATETIME,
    timeout_at DATETIME NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE UNIQUE INDEX idx_resource_locks_active ON resource_locks(
    project_id, resource_type, resource_key
) WHERE released_at IS NULL;

CREATE INDEX idx_resource_locks_timeout ON resource_locks(timeout_at) WHERE released_at IS NULL;
```

### Testing Requirements

- Lock acquisition tests
- Concurrent access tests
- Timeout behavior tests
- Deadlock detection tests
- RAII release tests

### References

- [Source: docs/PRD.md#Multi-Project-Concurrency] - Concurrency requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Multi-Project] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
