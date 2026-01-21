# Story 4.1: Create Automatic Checkpoints Before Major Changes

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically create a complete project state checkpoint before generating new code,
so that I can always recover if something goes wrong.

## Acceptance Criteria

1. **Given** User triggers code generation for a feature
   **When** Generation starts
   **Then** System creates checkpoint record in checkpoints table with project_id, feature_id, description, snapshot_data

2. **Given** Checkpoint is created
   **When** State is captured
   **Then** snapshot_data contains: JSON serialization of current project state (phases, features, chat_messages, generated_code)

3. **Given** Checkpoint is saved
   **When** Size is calculated
   **Then** Checkpoint size in bytes is recorded

4. **Given** Checkpoint is created
   **When** Security is applied
   **Then** Checkpoint is signed with ed25519 private key, signature stored

5. **Given** User views checkpoint list
   **When** Checkpoints are displayed
   **Then** Checkpoints show: timestamp, description (e.g., "Before generating User Auth"), size

6. **Given** Retention policy is active
   **When** Checkpoints accumulate
   **Then** Oldest checkpoints beyond retention limit are automatically deleted

## Tasks / Subtasks

- [ ] Task 1: Create checkpoints table schema (AC: #1, #3, #4)
  - [ ] Add migration for checkpoints table
  - [ ] Include columns: id, project_id, feature_id, description, snapshot_data, size_bytes, signature, created_at
  - [ ] Add indexes for project_id queries

- [ ] Task 2: Implement checkpoint creation service (AC: #1, #2)
  - [ ] Create CheckpointManager service
  - [ ] Capture project state as JSON
  - [ ] Include phases, features, chat_messages, generated_code
  - [ ] Calculate size in bytes

- [ ] Task 3: Implement ed25519 signing (AC: #4)
  - [ ] Add ed25519-dalek dependency
  - [ ] Create signature generation function
  - [ ] Store signature with checkpoint
  - [ ] Compile public key into binary

- [ ] Task 4: Integrate with code generation (AC: #1)
  - [ ] Hook into generation pipeline
  - [ ] Create checkpoint before generation starts
  - [ ] Add description: "Before generating {feature_name}"

- [ ] Task 5: Implement checkpoint list view (AC: #5)
  - [ ] Create list_checkpoints Tauri command
  - [ ] Return checkpoint metadata (no snapshot_data)
  - [ ] Format for display

- [ ] Task 6: Implement retention policy (AC: #6)
  - [ ] Create cleanup task
  - [ ] Configure retention_days (default 30)
  - [ ] Configure max_per_project (default 50)
  - [ ] Run cleanup periodically

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain service pattern for CheckpointManager
- Ed25519 signing for integrity verification
- Automatic triggering before major changes

**Security Requirements:**
- Checkpoint file signing with ed25519 for integrity verification
- Public key compiled into binary (const array)

### Checkpoint Data Structure

```rust
pub struct Checkpoint {
    pub id: String,
    pub project_id: String,
    pub feature_id: Option<String>,
    pub description: String,
    pub snapshot_data: serde_json::Value, // JSON blob
    pub size_bytes: i64,
    pub signature: Vec<u8>, // ed25519 signature
    pub created_at: DateTime<Utc>,
}

// Snapshot includes:
{
  "phases": [...],
  "features": [...],
  "chat_messages": [...],
  "generated_code": [...]
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/recovery/checkpoint.rs` - Checkpoint entity
- `crates/demiarch-core/src/domain/recovery/checkpoint_manager.rs` - Manager service
- `crates/demiarch-core/src/infrastructure/crypto/signing.rs` - Ed25519 operations
- `crates/demiarch-core/src/infrastructure/db/checkpoints.rs` - Repository

**Database Schema:**
```sql
CREATE TABLE checkpoints (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    feature_id TEXT,
    description TEXT NOT NULL,
    snapshot_data TEXT NOT NULL,  -- JSON blob
    size_bytes INTEGER NOT NULL,
    signature BLOB NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

### Testing Requirements

- Unit tests for checkpoint creation
- Signature generation/verification tests
- Retention policy tests
- Integration tests for generation pipeline hook

### References

- [Source: docs/PRD.md#Recovery-System] - Recovery requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Checkpoint-Security] - Security architecture
- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
