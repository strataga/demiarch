# Story 4.4: Display Conflict Resolution Interface

Status: ready-for-dev

## Story

As a user,
I want to see side-by-side diff when AI regenerates code I've edited,
so that I can choose what to keep intelligently.

## Acceptance Criteria

1. **Given** User has modified a generated file and AI regenerates it
   **When** Regeneration completes
   **Then** System detects conflict by comparing checksums

2. **Given** Conflict is detected
   **When** Status is updated
   **Then** generated_code.conflict_status is set to 'pending'

3. **Given** User views feature in GUI
   **When** Conflict exists
   **Then** Modal opens with side-by-side diff: original AI version vs new AI version

4. **Given** Diff is displayed
   **When** User's edits are shown
   **Then** User's edits are highlighted with amber overlay

5. **Given** Conflict resolution modal is open
   **When** Options are displayed
   **Then** Options displayed: "Keep My Changes", "Keep AI Version", "Merge Both", "Manual Edit"

6. **Given** Diff contains suspicious code
   **When** Security check runs
   **Then** Security annotations show for suspicious additions (e.g., "adds exec()", "adds eval()")

7. **Given** User selects "Keep My Changes"
   **When** Resolution is applied
   **Then** New AI code is discarded, user's version is kept

8. **Given** Conflict is resolved
   **When** Status is updated
   **Then** conflict_status is set to 'resolved_keep_user'

9. **Given** Resolution is complete
   **When** Record is created
   **Then** Record is added to conflict_resolutions table

## Tasks / Subtasks

- [ ] Task 1: Create conflict_resolutions table schema (AC: #9)
  - [ ] Add migration for conflict_resolutions table
  - [ ] Include columns: id, generated_code_id, resolution_type, user_content_before, ai_content, merged_content, resolved_at
  - [ ] Add foreign key to generated_code

- [ ] Task 2: Implement conflict detection (AC: #1, #2)
  - [ ] Check user_modified flag before regeneration
  - [ ] Compare checksums on regeneration
  - [ ] Set conflict_status = 'pending'
  - [ ] Store new AI content temporarily

- [ ] Task 3: Create ConflictResolutionModal component (AC: #3, #4, #5)
  - [ ] Create modal with side-by-side diff view
  - [ ] Highlight user changes with amber
  - [ ] Display resolution options as buttons
  - [ ] Use diff library (diff-match-patch or similar)

- [ ] Task 4: Implement security annotations (AC: #6)
  - [ ] Scan diff for suspicious patterns
  - [ ] Detect: exec(), eval(), dangerous imports
  - [ ] Display warning annotations inline
  - [ ] Color-code security warnings (red)

- [ ] Task 5: Implement resolution actions (AC: #7, #8)
  - [ ] Keep My Changes: discard AI, keep user
  - [ ] Keep AI Version: discard user, keep AI
  - [ ] Merge Both: open merge editor
  - [ ] Manual Edit: open full editor
  - [ ] Update conflict_status appropriately

- [ ] Task 6: Implement resolution recording (AC: #9)
  - [ ] Create conflict_resolutions record
  - [ ] Store all versions for history
  - [ ] Record resolution type and timestamp

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Side-by-side diff visualization
- Security annotation for suspicious code
- Multiple resolution strategies

**Security Requirements:**
- Clear diff with security annotations (e.g., "adds exec()")
- Flag suspicious code patterns

### Conflict Status Values

```typescript
type ConflictStatus =
  | 'none'              // No conflict
  | 'pending'           // Conflict detected, awaiting resolution
  | 'resolved_keep_user'  // User chose their version
  | 'resolved_keep_ai'    // User chose AI version
  | 'resolved_merged';    // User merged versions
```

### Security Patterns to Detect

```typescript
const SUSPICIOUS_PATTERNS = [
  /\bexec\s*\(/,           // exec()
  /\beval\s*\(/,           // eval()
  /\bFunction\s*\(/,       // Function constructor
  /require\s*\(['"](child_process|fs|net)/,  // Dangerous imports
  /import\s+.*from\s+['"](child_process|fs|net)/,
];
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/conflicts/ConflictResolutionModal.tsx`
- `crates/demiarch-gui/src/components/conflicts/DiffView.tsx`
- `crates/demiarch-gui/src/components/conflicts/SecurityAnnotation.tsx`
- `crates/demiarch-core/src/domain/code_generation/conflict_resolver.rs`

**Database Schema:**
```sql
CREATE TABLE conflict_resolutions (
    id TEXT PRIMARY KEY,
    generated_code_id TEXT NOT NULL,
    resolution_type TEXT NOT NULL CHECK(resolution_type IN ('keep_user', 'keep_ai', 'merged', 'manual')),
    user_content_before TEXT,
    ai_content TEXT,
    merged_content TEXT,
    resolved_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (generated_code_id) REFERENCES generated_code(id)
);
```

### Testing Requirements

- Conflict detection tests
- Diff generation tests
- Resolution action tests
- Security annotation tests

### References

- [Source: docs/PRD.md#Conflict-Resolution] - Conflict resolution requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Conflict-Detection] - Security annotations
- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
