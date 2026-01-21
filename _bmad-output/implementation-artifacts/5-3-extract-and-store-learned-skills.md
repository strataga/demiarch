# Story 5.3: Extract and Store Learned Skills

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically extract debugging solutions as reusable skills,
so that similar problems are solved faster in the future.

## Acceptance Criteria

1. **Given** An agent encounters and solves a technical problem (e.g., "Fixed TypeScript error with async/await")
   **When** Agent completes successfully
   **Then** System analyzes error and solution for reusability

2. **Given** Solution is reusable
   **When** Skill is extracted
   **Then** A learned_skills record is created

3. **Given** Skill is created
   **When** Fields are populated
   **Then** Fields populated: name, description, problem, trigger_conditions (JSON array), solution, verification, category='debugging'

4. **Given** Skill is new
   **When** Quality is initialized
   **Then** Quality score is initialized to 0.0

5. **Given** Skill is stored
   **When** Embedding is generated
   **Then** Embedding is generated for solution (1536-dimensional using OpenAI text-embedding-3-small)

6. **Given** Embedding is generated
   **When** Embedding is stored
   **Then** Embedding is stored in skill_embeddings table with embedding_model

7. **Given** User views learned skills
   **When** Skills are displayed
   **Then** Skills display: name, description, problem, usage_count, quality_score

8. **Given** User reviews skill
   **When** User marks as verified
   **Then** User can mark skill as "Verified" to increase quality score

## Tasks / Subtasks

- [ ] Task 1: Create learned_skills table schema (AC: #2, #3, #4)
  - [ ] Add migration for learned_skills table
  - [ ] Include columns: id, name, description, problem, trigger_conditions, solution, verification, category, quality_score, usage_count, is_verified
  - [ ] Initialize quality_score to 0.0

- [ ] Task 2: Create skill_embeddings table schema (AC: #5, #6)
  - [ ] Add migration for skill_embeddings table
  - [ ] Include columns: id, skill_id, embedding, embedding_model
  - [ ] Use sqlite-vec for vector storage

- [ ] Task 3: Implement skill extraction service (AC: #1, #2, #3)
  - [ ] Create SkillExtractor service
  - [ ] Analyze agent execution for error patterns
  - [ ] Check quality gate criteria
  - [ ] Extract structured skill using LLM

- [ ] Task 4: Implement quality gate (AC: #1)
  - [ ] Check investigation depth >= 3 turns
  - [ ] Check has error pattern
  - [ ] Check has verification step
  - [ ] Only extract if criteria met

- [ ] Task 5: Implement embedding generation (AC: #5, #6)
  - [ ] Generate embedding from description + problem + trigger_conditions
  - [ ] Use text-embedding-3-small (1536 dimensions)
  - [ ] Store in skill_embeddings table

- [ ] Task 6: Implement skills list UI (AC: #7, #8)
  - [ ] Create SkillsList component
  - [ ] Display skill metadata
  - [ ] Add "Verify" button
  - [ ] Update quality_score on verification

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Autonomous skill extraction with quality gate
- Embedding-based semantic search
- RL quality optimization via feedback

**Skill Requirements:**
- Skills require quality gate check before extraction
- Manual approval or rate-limit with RL feedback

### Quality Gate Criteria

```rust
pub struct QualityGate {
    pub min_investigation_turns: u32,  // >= 3
    pub requires_error_pattern: bool,   // true
    pub requires_verification: bool,    // true
}

pub fn passes_quality_gate(execution: &AgentExecution) -> bool {
    let turns = execution.turns.len();
    let has_error = execution.context.contains_error_pattern();
    let has_verification = execution.has_verification_step();

    turns >= 3 && has_error && has_verification
}
```

### Skill Structure

```rust
pub struct LearnedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub problem: String,
    pub trigger_conditions: Vec<String>,  // JSON array
    pub solution: String,
    pub verification: String,
    pub category: String,  // 'debugging', 'optimization', etc.
    pub quality_score: f64,  // 0.0 to 1.0
    pub usage_count: u32,
    pub is_verified: bool,
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/skills/skill.rs`
- `crates/demiarch-core/src/domain/skills/extractor.rs`
- `crates/demiarch-core/src/domain/skills/quality_gate.rs`
- `crates/demiarch-core/src/infrastructure/db/learned_skills.rs`
- `crates/demiarch-gui/src/components/skills/SkillsList.tsx`

**Database Schema:**
```sql
CREATE TABLE learned_skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    problem TEXT NOT NULL,
    trigger_conditions TEXT NOT NULL,  -- JSON array
    solution TEXT NOT NULL,
    verification TEXT,
    category TEXT DEFAULT 'debugging',
    quality_score REAL DEFAULT 0.0,
    usage_count INTEGER DEFAULT 0,
    is_verified INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE skill_embeddings (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL,
    embedding BLOB NOT NULL,
    embedding_model TEXT NOT NULL,
    FOREIGN KEY (skill_id) REFERENCES learned_skills(id)
);
```

### Testing Requirements

- Quality gate logic tests
- Skill extraction tests
- Embedding generation tests
- Verification flow tests

### References

- [Source: docs/PRD.md#Learned-Skills-Extraction] - Skill extraction requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Agent-System-Resilience] - Quality gate details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
