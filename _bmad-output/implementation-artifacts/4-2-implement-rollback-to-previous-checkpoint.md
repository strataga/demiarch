# Story 4.2: Implement Rollback to Previous Checkpoint

Status: ready-for-dev

## Story

As a user,
I want to restore my project to a previous checkpoint with one click,
so that I can recover from mistakes or failed generations.

## Acceptance Criteria

1. **Given** User has one or more checkpoints and opens recovery UI
   **When** User selects a checkpoint and clicks "Restore"
   **Then** System verifies checkpoint signature using ed25519 public key

2. **Given** Signature verification fails
   **When** Restore is attempted
   **Then** User sees error: "Checkpoint signature verification failed"

3. **Given** Signature is valid
   **When** Restore proceeds
   **Then** snapshot_data is deserialized and applied to project state

4. **Given** Restore is in progress
   **When** Tables are updated
   **Then** All tables (phases, features, chat_messages, generated_code) are restored from snapshot

5. **Given** Restore completes successfully
   **When** Files are restored
   **Then** Current generated code files are reverted to checkpoint versions

6. **Given** Restore completes
   **When** User receives feedback
   **Then** User sees success: "Project restored to state from [timestamp]"

7. **Given** Restore operation starts
   **When** Performance is measured
   **Then** Restore completes within 5 seconds

8. **Given** Restore completes
   **When** Safety backup is created
   **Then** Checkpoint creation happens after restore (for safety)

## Tasks / Subtasks

- [ ] Task 1: Implement signature verification (AC: #1, #2)
  - [ ] Create verify_signature function
  - [ ] Load public key from compiled constant
  - [ ] Return verification result
  - [ ] Handle verification failure with clear error

- [ ] Task 2: Implement state restoration (AC: #3, #4)
  - [ ] Create restore_checkpoint service method
  - [ ] Deserialize snapshot_data JSON
  - [ ] Clear existing project data
  - [ ] Insert restored data into all tables

- [ ] Task 3: Implement file restoration (AC: #5)
  - [ ] Track generated files from checkpoint
  - [ ] Revert file contents to checkpoint versions
  - [ ] Handle missing files (recreate)
  - [ ] Handle extra files (optionally delete)

- [ ] Task 4: Create restore UI (AC: #6)
  - [ ] Create CheckpointList component
  - [ ] Add Restore button per checkpoint
  - [ ] Show confirmation dialog
  - [ ] Display success/error message

- [ ] Task 5: Optimize restore performance (AC: #7)
  - [ ] Use transactions for database operations
  - [ ] Batch file operations
  - [ ] Ensure under 5 seconds

- [ ] Task 6: Implement safety backup (AC: #8)
  - [ ] Create checkpoint before restore
  - [ ] Description: "Auto-backup before restore"
  - [ ] Allow rollback of rollback

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Ed25519 signature verification before restore
- Transactional database operations
- Safety backup before destructive operation

**Security Requirements:**
- Verify checkpoint signature before restore
- Public key compiled into binary (not loaded from disk)

### Restore Flow

```
User clicks Restore
    ↓
Verify ed25519 signature
    ↓ (fail → show error)
Create safety backup checkpoint
    ↓
Begin database transaction
    ↓
Clear existing project data
    ↓
Insert snapshot data
    ↓
Commit transaction
    ↓
Restore files to disk
    ↓
Show success message
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/recovery/restore.rs` - Restore service
- `crates/demiarch-core/src/infrastructure/crypto/verify.rs` - Verification
- `crates/demiarch-gui/src/components/recovery/CheckpointList.tsx`
- `crates/demiarch-gui/src/components/recovery/RestoreDialog.tsx`

**Public Key Storage:**
```rust
// In crates/demiarch-core/src/infrastructure/crypto/keys.rs
pub const CHECKPOINT_PUBLIC_KEY: [u8; 32] = [
    // Compiled-in public key bytes
];
```

### Testing Requirements

- Signature verification tests (valid and invalid)
- Full restore integration tests
- File restoration tests
- Performance tests (under 5 seconds)
- Safety backup verification

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Checkpoint-Security] - Security requirements
- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
