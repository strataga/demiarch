# Story 2.3: Generate Code from Natural Language Description

Status: ready-for-dev

## Story

As a user,
I want to describe a feature in natural language and have AI generate complete, working code,
so that I can build applications without writing code manually.

## Acceptance Criteria

1. **Given** User is in a chat conversation and describes a feature (e.g., "Add user authentication with login and logout")
   **When** AI processes the request and generates code
   **Then** Generated files are created in the project repository at specified paths

2. **Given** Code is generated
   **When** Files are created
   **Then** Each file is recorded in generated_code table with feature_id, file_path, content, original_checksum, current_checksum

3. **Given** Code generation completes
   **When** Files are written
   **Then** Files are created using the project's configured framework (Next.js, React, Flutter, etc.)

4. **Given** Generation is successful
   **When** User receives feedback
   **Then** User sees message: "Generated 5 files for user authentication feature"

5. **Given** Generated code is complete
   **When** User runs the code
   **Then** User can run the generated code and it functions as described

## Tasks / Subtasks

- [ ] Task 1: Create generated_code table schema (AC: #2)
  - [ ] Add migration for generated_code table
  - [ ] Include columns: id, feature_id, file_path, content, language, original_checksum, current_checksum, user_modified, conflict_status
  - [ ] Add indexes for feature_id queries

- [ ] Task 2: Implement code generation domain service (AC: #1, #3)
  - [ ] Create CodeGenerator trait
  - [ ] Implement feature parsing from natural language
  - [ ] Generate framework-appropriate code structure
  - [ ] Support multiple frameworks (Next.js, React, etc.)

- [ ] Task 3: Implement file writing service (AC: #1, #2)
  - [ ] Create FileWriter service
  - [ ] Calculate checksums (SHA-256) for files
  - [ ] Write files to repository
  - [ ] Record files in generated_code table

- [ ] Task 4: Create generation prompts (AC: #3, #5)
  - [ ] Design system prompt for code generation
  - [ ] Include framework-specific instructions
  - [ ] Add best practices and patterns
  - [ ] Ensure generated code is runnable

- [ ] Task 5: Implement generation feedback (AC: #4)
  - [ ] Count generated files
  - [ ] Summarize generated components
  - [ ] Display user-friendly message
  - [ ] Include file list in response

- [ ] Task 6: Wire generation to chat interface
  - [ ] Detect code generation requests
  - [ ] Trigger generation pipeline
  - [ ] Update UI with progress
  - [ ] Show completion message

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain service pattern for CodeGenerator
- Repository pattern for generated_code storage
- Checksum tracking for conflict detection

**Code Generation Requirements:**
- Support all major frameworks from PRD
- Generate complete, runnable code
- Track all generated files for conflict resolution

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/code_generation/` - Code generation domain
- `crates/demiarch-core/src/infrastructure/fs/` - File system operations
- `crates/demiarch-core/src/application/commands/generate.rs` - Generation command

**Database Schema:**
```sql
CREATE TABLE generated_code (
    id TEXT PRIMARY KEY,
    feature_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content TEXT NOT NULL,
    language TEXT,
    original_checksum TEXT NOT NULL,
    current_checksum TEXT NOT NULL,
    user_modified INTEGER DEFAULT 0,
    conflict_status TEXT DEFAULT 'none',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (feature_id) REFERENCES features(id)
);
```

### Framework Support

Initial frameworks to support:
- Next.js (React + TypeScript)
- React + Vite
- Vue 3
- SvelteKit

### Testing Requirements

- Unit tests for code generation prompts
- Integration tests for file writing
- E2E tests for complete generation flow
- Framework-specific generation tests

### References

- [Source: docs/PRD.md#AI-Powered-Code-Generation] - Code generation requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Code-Generation] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
