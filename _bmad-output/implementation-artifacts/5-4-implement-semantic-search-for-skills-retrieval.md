# Story 5.4: Implement Semantic Search for Skills Retrieval

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically suggest relevant learned skills when I encounter a problem,
so that I can apply proven solutions without waiting for AI.

## Acceptance Criteria

1. **Given** User has learned skills stored and encounters a new error
   **When** Error is detected or user describes problem in chat
   **Then** System generates embedding for error/problem description

2. **Given** Embedding is generated
   **When** Search is performed
   **Then** System queries skill_embeddings table for KNN search (sqlite-vec)

3. **Given** KNN search completes
   **When** Results are returned
   **Then** Top 3 most similar skills are returned (by cosine distance)

4. **Given** Skills are retrieved
   **When** Security check runs
   **Then** System applies semantic filter to retrieved skills (scans for prompt injection patterns)

5. **Given** Filter detects suspicious content
   **When** Skill is flagged
   **Then** Skill is not suggested, event logged in audit_log with event_type='prompt_injection_attempt'

6. **Given** No injection detected
   **When** Skills are suggested
   **Then** Skills are suggested to user: "Similar problem found in 3 skills. Apply this solution?"

7. **Given** User interacts with suggestion
   **When** User applies or declines
   **Then** User can click to apply skill or decline suggestion

8. **Given** Skill is applied
   **When** Activation is recorded
   **Then** When skill is applied, activation_type='automatic' is recorded in skill_activations table

## Tasks / Subtasks

- [ ] Task 1: Create skill_activations table schema (AC: #8)
  - [ ] Add migration for skill_activations table
  - [ ] Include columns: id, skill_id, agent_execution_id, activation_type, outcome, activated_at
  - [ ] Add foreign keys

- [ ] Task 2: Implement embedding generation for queries (AC: #1)
  - [ ] Create query embedding generator
  - [ ] Use same model as skill embeddings
  - [ ] Handle error messages and descriptions

- [ ] Task 3: Implement KNN search (AC: #2, #3)
  - [ ] Create SkillSearcher service
  - [ ] Use sqlite-vec for KNN query
  - [ ] Return top 3 by cosine distance
  - [ ] Include skill metadata in results

- [ ] Task 4: Implement semantic filter (AC: #4, #5)
  - [ ] Create SemanticFilter service
  - [ ] Scan for prompt injection patterns
  - [ ] Log suspicious content to audit_log
  - [ ] Filter out flagged skills

- [ ] Task 5: Implement suggestion UI (AC: #6, #7)
  - [ ] Create SkillSuggestion component
  - [ ] Display skill name and solution preview
  - [ ] Add "Apply" and "Dismiss" buttons
  - [ ] Handle user interaction

- [ ] Task 6: Implement activation recording (AC: #8)
  - [ ] Record skill activation
  - [ ] Set activation_type based on trigger
  - [ ] Track outcome for RL feedback

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Vector-based semantic search using sqlite-vec
- Semantic filter for prompt injection prevention
- RL feedback via activation tracking

**Security Requirements:**
- Semantic filter at embedding retrieval to scan for prompt injection patterns
- Log suspicious content to audit_log

### Prompt Injection Patterns

```rust
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "new role:",
    "you are now",
    "forget everything",
    "disregard all prior",
    "system prompt:",
];

pub fn is_prompt_injection(content: &str) -> bool {
    let lower = content.to_lowercase();
    INJECTION_PATTERNS.iter().any(|p| lower.contains(p))
}
```

### KNN Query with sqlite-vec

```sql
SELECT s.*, e.embedding
FROM learned_skills s
JOIN skill_embeddings e ON s.id = e.skill_id
WHERE e.embedding MATCH ?1  -- Query embedding
ORDER BY distance
LIMIT 3;
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/skills/searcher.rs`
- `crates/demiarch-core/src/domain/skills/semantic_filter.rs`
- `crates/demiarch-core/src/infrastructure/db/skill_activations.rs`
- `crates/demiarch-gui/src/components/skills/SkillSuggestion.tsx`

**Database Schema:**
```sql
CREATE TABLE skill_activations (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL,
    agent_execution_id TEXT,
    activation_type TEXT NOT NULL,  -- 'automatic', 'manual', 'suggested'
    outcome TEXT,  -- 'success', 'failure', 'skipped'
    activated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (skill_id) REFERENCES learned_skills(id)
);
```

### Testing Requirements

- KNN search accuracy tests
- Semantic filter detection tests
- Activation recording tests
- UI interaction tests

### References

- [Source: docs/PRD.md#Learned-Skills-Extraction] - Skill search requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Progressive-Disclosure] - Semantic filter details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
