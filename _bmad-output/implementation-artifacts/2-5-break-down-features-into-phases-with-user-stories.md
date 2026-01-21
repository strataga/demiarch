# Story 2.5: Break Down Features into Phases with User Stories

Status: ready-for-dev

## Story

As a user,
I want to automatically break down my project into phases with user stories and acceptance criteria,
so that I can track development progress in a structured way.

## Acceptance Criteria

1. **Given** User has discussed project scope and requests phase planning
   **When** AI analyzes requirements and generates phase breakdown
   **Then** Phase records are created in phases table with name, description, status, order_index

2. **Given** Phases are created
   **When** Features are assigned
   **Then** Feature records are created in features table linked to phase_id with name, description, acceptance_criteria, priority

3. **Given** Phase breakdown is complete
   **When** User views phases
   **Then** User sees organized phases: "Discovery", "Planning", "Building", "Complete"

4. **Given** Phases are displayed
   **When** User views phase details
   **Then** Each phase shows count of features within it

5. **Given** Features exist in phases
   **When** User interacts with kanban board (Epic 3)
   **Then** User can drag features between phases

## Tasks / Subtasks

- [ ] Task 1: Create phases table schema (AC: #1)
  - [ ] Add migration for phases table
  - [ ] Include columns: id, project_id, name, description, status, order_index
  - [ ] Add default phases (Discovery, Planning, Building, Complete)
  - [ ] Support status: pending, in_progress, complete, skipped

- [ ] Task 2: Create features table schema (AC: #2)
  - [ ] Add migration for features table
  - [ ] Include columns: id, phase_id, name, description, acceptance_criteria, priority, status, labels
  - [ ] Support status: pending, ready, in_progress, complete
  - [ ] Add priority range (1-5)

- [ ] Task 3: Implement phase planning service (AC: #1, #3)
  - [ ] Create PhasePlanner service
  - [ ] Parse project scope from conversation
  - [ ] Generate appropriate phases
  - [ ] Create default phase structure

- [ ] Task 4: Implement feature extraction service (AC: #2)
  - [ ] Create FeatureExtractor service
  - [ ] Extract features from requirements
  - [ ] Generate acceptance criteria
  - [ ] Assign to appropriate phases

- [ ] Task 5: Implement repositories (AC: #4)
  - [ ] Create PhaseRepository
  - [ ] Create FeatureRepository
  - [ ] Implement CRUD operations
  - [ ] Add feature count queries

- [ ] Task 6: Wire to chat interface
  - [ ] Detect phase planning requests
  - [ ] Trigger planning pipeline
  - [ ] Display generated phases and features
  - [ ] Confirm before saving

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain service pattern for PhasePlanner and FeatureExtractor
- Repository pattern for data access
- Aggregate root pattern (Phase contains Features)

**Planning Requirements:**
- Default 4-phase structure
- Automatic feature extraction
- Acceptance criteria generation
- Priority assignment

### Default Phases

1. **Discovery** (order_index: 0) - Requirements gathering and ideation
2. **Planning** (order_index: 1) - Technical design and architecture
3. **Building** (order_index: 2) - Implementation and development
4. **Complete** (order_index: 3) - Finished and deployed

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/projects/phase.rs` - Phase entity
- `crates/demiarch-core/src/domain/projects/feature.rs` - Feature entity
- `crates/demiarch-core/src/infrastructure/db/phases.rs` - Phase repository
- `crates/demiarch-core/src/infrastructure/db/features.rs` - Feature repository
- `crates/demiarch-core/src/application/planners/` - Planning services

**Database Schema:**
```sql
CREATE TABLE phases (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'in_progress', 'complete', 'skipped')),
    order_index INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE features (
    id TEXT PRIMARY KEY,
    phase_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    acceptance_criteria TEXT,
    priority INTEGER DEFAULT 3 CHECK(priority BETWEEN 1 AND 5),
    status TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'ready', 'in_progress', 'complete')),
    labels TEXT,  -- JSON array
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (phase_id) REFERENCES phases(id)
);
```

### Testing Requirements

- Unit tests for phase planning logic
- Unit tests for feature extraction
- Integration tests for repository operations
- E2E tests for complete planning flow

### References

- [Source: docs/PRD.md#Phase-Planning] - Phase planning requirements
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Kanban-Board] - Phase visualization
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
