# Story 1.4: Implement Basic CLI Interface

Status: ready-for-dev

## Story

As a developer user,
I want to interact with Demiarch through a command-line interface,
so that I can script operations and use terminal workflows.

## Acceptance Criteria

1. **Given** Demiarch is installed and database is initialized
   **When** User runs `demiarch --help`
   **Then** CLI displays available commands: project, generate, sync, config, watch

2. **Given** CLI help is displayed
   **When** User views command list
   **Then** Each command shows usage and required arguments

3. **Given** User runs `demiarch project list`
   **When** Projects exist in database
   **Then** All projects are displayed in a table format with ID, name, status, framework

4. **Given** User runs `demiarch project list`
   **When** No projects exist
   **Then** Empty state shows "No projects found. Create one with `demiarch project create`"

5. **Given** Any CLI command encounters an error
   **When** Error is displayed
   **Then** Errors display with user-friendly messages and actionable suggestions

## Tasks / Subtasks

- [ ] Task 1: Setup CLI binary structure (AC: #1)
  - [ ] Configure clap 4.5 with derive macros
  - [ ] Define top-level CLI struct with subcommands
  - [ ] Add global flags (--help, --version, --json, --quiet, --verbose)

- [ ] Task 2: Implement project subcommand (AC: #2, #3, #4)
  - [ ] Create ProjectCommand enum with subcommands (list, create, switch, archive, delete)
  - [ ] Implement `project list` command
  - [ ] Add table formatting using tabled or comfy-table
  - [ ] Handle empty state display

- [ ] Task 3: Implement config subcommand (AC: #2)
  - [ ] Create ConfigCommand enum (get, set, reset, list)
  - [ ] Implement config get/set commands
  - [ ] Add special handling for sensitive keys (set-api-key)

- [ ] Task 4: Stub remaining subcommands (AC: #1, #2)
  - [ ] Create GenerateCommand placeholder
  - [ ] Create SyncCommand placeholder
  - [ ] Create WatchCommand placeholder

- [ ] Task 5: Implement error handling (AC: #5)
  - [ ] Create CLI error types wrapping DemiarchError
  - [ ] Implement Display trait with user-friendly messages
  - [ ] Add suggestion() method integration
  - [ ] Add error code display

- [ ] Task 6: Wire CLI to core library
  - [ ] Import and use demiarch-core commands
  - [ ] Ensure CLI calls library functions (CLI-as-Library pattern)
  - [ ] Add async runtime setup (tokio)

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- CLI-as-Library pattern: All commands as reusable library functions
- Use clap 4.5 for argument parsing with derive macros
- Share command implementations with GUI through demiarch-core

**Technology Stack:**
- clap 4.5 for CLI parsing
- tokio 1.41 for async runtime
- tabled or comfy-table for table output
- demiarch-core for all business logic

### Project Structure Notes

**File Locations:**
- `crates/demiarch-cli/src/main.rs` - Entry point
- `crates/demiarch-cli/src/commands/` - Command implementations
- `crates/demiarch-cli/src/output/` - Output formatting
- `crates/demiarch-core/src/application/commands/` - Shared command logic

**Naming Conventions:**
- Commands: verb-noun (e.g., `project_list`, `config_set`)
- Subcommands: lowercase with hyphens in CLI (`demiarch project list`)

### Testing Requirements

- Unit tests for argument parsing
- Integration tests for each command
- Snapshot tests for help output
- Error message formatting tests

### References

- [Source: docs/PRD.md#CLI-Commands] - CLI command specifications
- [Source: _bmad-output/planning-artifacts/architecture.md#CLI-as-Library] - Architecture pattern
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
