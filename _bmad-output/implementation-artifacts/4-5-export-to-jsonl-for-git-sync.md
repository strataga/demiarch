# Story 4.5: Export to JSONL for Git Sync

Status: ready-for-dev

## Story

As a user,
I want to explicitly export my project data to JSONL format for git version control,
so that I can commit my project history without automatic sync.

## Acceptance Criteria

1. **Given** User has a project and wants to sync to git
   **When** User runs `demiarch sync --flush-only`
   **Then** All tables are exported to .demiarch/{project_id}/export.jsonl

2. **Given** Export is performed
   **When** Data is written
   **Then** Each record is one JSON line in JSONL format

3. **Given** Export completes
   **When** Metadata is updated
   **Then** metadata.dirty flag is set to 0

4. **Given** Export is successful
   **When** User receives feedback
   **Then** User sees: "Exported to export.jsonl. Run 'git add .demiarch/ && git commit' to save."

5. **Given** User runs `demiarch sync --import-only`
   **When** Import is performed
   **Then** Data from export.jsonl is imported to SQLite database

6. **Given** Import is performed
   **When** Validation runs
   **Then** Each line is validated against JSON schema before import

7. **Given** Import completes
   **When** Data is imported
   **Then** Import completes without data corruption

8. **Given** Import is successful
   **When** User receives feedback
   **Then** User sees: "Imported N records from export.jsonl"

## Tasks / Subtasks

- [ ] Task 1: Create metadata table with dirty flag (AC: #3)
  - [ ] Add metadata table if not exists
  - [ ] Include dirty flag (0 = synced, 1 = dirty)
  - [ ] Track last_sync_at timestamp

- [ ] Task 2: Implement JSONL export service (AC: #1, #2)
  - [ ] Create SyncService with export method
  - [ ] Query all tables for project
  - [ ] Write each record as JSON line
  - [ ] Output to .demiarch/{project_id}/export.jsonl

- [ ] Task 3: Implement export CLI command (AC: #1, #4)
  - [ ] Create `demiarch sync --flush-only` command
  - [ ] Call SyncService.export()
  - [ ] Update dirty flag to 0
  - [ ] Display success message with git instructions

- [ ] Task 4: Implement JSON schema validation (AC: #6)
  - [ ] Define JSON schema for each table type
  - [ ] Validate each line before import
  - [ ] Report validation errors clearly

- [ ] Task 5: Implement JSONL import service (AC: #5, #7)
  - [ ] Create import method in SyncService
  - [ ] Read JSONL file line by line
  - [ ] Validate each record
  - [ ] Insert into database with conflict handling

- [ ] Task 6: Implement import CLI command (AC: #5, #8)
  - [ ] Create `demiarch sync --import-only` command
  - [ ] Call SyncService.import()
  - [ ] Display record count on success
  - [ ] Handle import errors gracefully

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Explicit sync operations (no automatic)
- JSONL format for git-friendly diffs
- Schema validation before import

**Storage Requirements:**
- SQLite primary storage with JSONL git-sync format
- JSONL sync: validate each entry against schema before writing

### JSONL Format

```jsonl
{"_table": "phases", "id": "...", "project_id": "...", "name": "Discovery", ...}
{"_table": "phases", "id": "...", "project_id": "...", "name": "Planning", ...}
{"_table": "features", "id": "...", "phase_id": "...", "name": "User Auth", ...}
{"_table": "chat_messages", "id": "...", "project_id": "...", "role": "user", ...}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/sync/sync_service.rs`
- `crates/demiarch-core/src/domain/sync/jsonl.rs`
- `crates/demiarch-core/src/domain/sync/schema.rs`
- `crates/demiarch-cli/src/commands/sync.rs`

**Export Path:**
```
.demiarch/
  {project_id}/
    export.jsonl       # All project data
    database.sqlite    # SQLite database
```

**Schema Validation:**
```rust
pub fn validate_record(line: &str) -> Result<ValidatedRecord> {
    let record: serde_json::Value = serde_json::from_str(line)?;
    let table = record["_table"].as_str()?;

    match table {
        "phases" => validate_phase(&record)?,
        "features" => validate_feature(&record)?,
        "chat_messages" => validate_chat_message(&record)?,
        // ...
    }
}
```

### Testing Requirements

- Export format tests
- Import validation tests
- Round-trip tests (export then import)
- Error handling tests

### References

- [Source: docs/PRD.md#Storage-Pattern] - JSONL sync requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#File-System-Integrity] - Validation requirements
- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
