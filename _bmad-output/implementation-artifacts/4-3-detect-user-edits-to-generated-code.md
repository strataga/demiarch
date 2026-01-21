# Story 4.3: Detect User Edits to Generated Code

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically detect when I edit AI-generated files,
so that conflicts are flagged when AI regenerates that code.

## Acceptance Criteria

1. **Given** User has generated code files recorded in generated_code table
   **When** User edits a file in their editor
   **Then** Demiarch watch system detects file change (via filesystem watcher)

2. **Given** File change is detected
   **When** Checksum is calculated
   **Then** New checksum is calculated for file (SHA-256)

3. **Given** New checksum is available
   **When** Database is updated
   **Then** generated_code.current_checksum is updated with new checksum

4. **Given** Checksum differs from original
   **When** User modified flag is set
   **Then** generated_code.user_modified is set to 1

5. **Given** File was edited
   **When** Edit is recorded
   **Then** Record is added to user_code_edits table with edit_type='modify', new_content, detected_at

6. **Given** User views code in GUI
   **When** Modified files are displayed
   **Then** Modified files show visual indicator (e.g., amber dot or "Edited" badge)

## Tasks / Subtasks

- [ ] Task 1: Create user_code_edits table schema (AC: #5)
  - [ ] Add migration for user_code_edits table
  - [ ] Include columns: id, generated_code_id, edit_type, new_content, detected_at
  - [ ] Add foreign key to generated_code

- [ ] Task 2: Implement filesystem watcher (AC: #1)
  - [ ] Add notify crate dependency
  - [ ] Create FileWatcher service
  - [ ] Watch generated code file paths
  - [ ] Debounce rapid changes

- [ ] Task 3: Implement checksum calculation (AC: #2)
  - [ ] Add sha2 crate (already in stack)
  - [ ] Create calculate_checksum function
  - [ ] Handle large files efficiently

- [ ] Task 4: Implement change detection logic (AC: #3, #4)
  - [ ] Compare current_checksum with file checksum
  - [ ] Update current_checksum on change
  - [ ] Set user_modified = 1 if differs from original

- [ ] Task 5: Implement edit recording (AC: #5)
  - [ ] Create UserCodeEditRepository
  - [ ] Record edit with new content
  - [ ] Set edit_type and detected_at

- [ ] Task 6: Implement GUI indicator (AC: #6)
  - [ ] Add "Edited" badge to file list
  - [ ] Use amber color for modified files
  - [ ] Query user_modified flag

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Filesystem watching with notify crate
- Checksum tracking using SHA-256
- Event-driven detection

**Change Detection Requirements:**
- Efficient conflict detection using file checksums
- Track all user modifications for conflict resolution

### File Watcher Design

```rust
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    debounce_duration: Duration,
    pending_changes: HashMap<PathBuf, Instant>,
}

impl FileWatcher {
    pub fn watch_generated_files(&self, file_paths: &[PathBuf]) -> Result<()>;
    pub fn on_file_change(&self, path: &Path) -> Result<ChangeEvent>;
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/code_generation/file_watcher.rs`
- `crates/demiarch-core/src/domain/code_generation/checksum.rs`
- `crates/demiarch-core/src/infrastructure/db/user_code_edits.rs`

**Database Schema:**
```sql
CREATE TABLE user_code_edits (
    id TEXT PRIMARY KEY,
    generated_code_id TEXT NOT NULL,
    edit_type TEXT NOT NULL CHECK(edit_type IN ('create', 'modify', 'delete')),
    new_content TEXT,
    detected_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (generated_code_id) REFERENCES generated_code(id)
);
```

### Testing Requirements

- File watcher event tests
- Checksum calculation tests
- Change detection logic tests
- GUI indicator tests

### References

- [Source: docs/PRD.md#Conflict-Resolution] - Conflict detection requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Conflict-Detection] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
