# Story 6.3: Cross-Project Search with Opt-In

Status: ready-for-dev

## Story

As a user with opt-in enabled,
I want to search across multiple projects to find code patterns and reuse solutions,
so that I can leverage work from other projects.

## Acceptance Criteria

1. **Given** User has enabled cross-project search in settings and has 2+ projects
   **When** User enters search query in GUI or CLI
   **Then** System searches selected projects' features, chat_messages, generated_code tables

2. **Given** Search completes
   **When** Results are returned
   **Then** Results show project name, feature name, relevance score

3. **Given** Results are displayed
   **When** User views results
   **Then** User sees which project each result came from

4. **Given** User clicks on a result
   **When** Result is selected
   **Then** Context is loaded from source project

5. **Given** Cross-project reference is made
   **When** Reference is recorded
   **Then** Reference is created in cross_project_refs table with source_project_id, target_project_id, ref_type

6. **Given** User wants full details
   **When** User navigates to source
   **Then** User can navigate to source project to view full details

7. **Given** Privacy concerns
   **When** Search settings are configured
   **Then** Search can be disabled per-project in settings

## Tasks / Subtasks

- [ ] Task 1: Create cross_project_refs table schema (AC: #5)
  - [ ] Add migration for cross_project_refs table
  - [ ] Include columns: id, source_project_id, target_project_id, ref_type, ref_id, created_at
  - [ ] Add indexes for project lookups

- [ ] Task 2: Add cross-project search setting (AC: #1, #7)
  - [ ] Add cross_project_search_enabled to user_preferences
  - [ ] Add project-level setting for search inclusion
  - [ ] Default to opt-out (false)

- [ ] Task 3: Implement CrossProjectSearcher service (AC: #1, #2, #3)
  - [ ] Create search service with multi-project support
  - [ ] Query features, chat_messages, generated_code tables
  - [ ] Calculate relevance scores
  - [ ] Filter by enabled projects

- [ ] Task 4: Implement search results UI (AC: #2, #3, #6)
  - [ ] Display results with project attribution
  - [ ] Show relevance scores
  - [ ] Add navigation to source project

- [ ] Task 5: Implement reference tracking (AC: #4, #5)
  - [ ] Create reference on result selection
  - [ ] Track source and target projects
  - [ ] Record reference type (code, feature, chat)

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Opt-in cross-project search for privacy
- Reference tracking for audit trail
- Multi-database queries (one SQLite per project)

**Security Requirements:**
- Search disabled by default (opt-in)
- Per-project search inclusion settings
- Audit trail via cross_project_refs

### Search Result Structure

```rust
pub struct CrossProjectSearchResult {
    pub project_id: String,
    pub project_name: String,
    pub result_type: SearchResultType,  // Feature, ChatMessage, Code
    pub result_id: String,
    pub title: String,
    pub snippet: String,
    pub relevance_score: f64,
}

pub enum RefType {
    Code,
    Feature,
    Chat,
    Skill,
}

pub struct CrossProjectRef {
    pub id: String,
    pub source_project_id: String,
    pub target_project_id: String,
    pub ref_type: RefType,
    pub ref_id: String,
    pub created_at: DateTime<Utc>,
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/search/cross_project.rs`
- `crates/demiarch-core/src/infrastructure/db/cross_project_refs.rs`
- `crates/demiarch-gui/src/components/search/CrossProjectResults.tsx`

**Database Schema:**
```sql
CREATE TABLE cross_project_refs (
    id TEXT PRIMARY KEY,
    source_project_id TEXT NOT NULL,
    target_project_id TEXT NOT NULL,
    ref_type TEXT NOT NULL,  -- 'code', 'feature', 'chat', 'skill'
    ref_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_project_id) REFERENCES projects(id),
    FOREIGN KEY (target_project_id) REFERENCES projects(id)
);

CREATE INDEX idx_cross_project_refs_source ON cross_project_refs(source_project_id);
CREATE INDEX idx_cross_project_refs_target ON cross_project_refs(target_project_id);
```

### Testing Requirements

- Search across multiple projects tests
- Opt-in/opt-out behavior tests
- Reference tracking tests
- Project isolation tests

### References

- [Source: docs/PRD.md#Multi-Project-Concurrency] - Cross-project requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Multi-Project] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
