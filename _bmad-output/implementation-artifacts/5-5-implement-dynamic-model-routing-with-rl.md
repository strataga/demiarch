# Story 5.5: Implement Dynamic Model Routing with RL

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically select the best model for each task type,
so that I get optimal performance at lowest cost.

## Acceptance Criteria

1. **Given** User triggers a task (code generation, debugging, math calculation)
   **When** System determines task_type
   **Then** System queries model_routing_rules table for matching task types

2. **Given** Routing rules are queried
   **When** Models are ranked
   **Then** Priority-ordered models are returned based on: is_specialized flag, min_quality_score, max_cost_per_1k

3. **Given** Model is selected
   **When** Decision is recorded
   **Then** Selection is recorded in routing_decisions table with: agent_execution_id, task_type, selected_model, selection_reason

4. **Given** Decision is recorded
   **When** Alternatives are captured
   **Then** Alternatives considered are stored as JSON array

5. **Given** Task completes
   **When** Performance is measured
   **Then** Performance metrics are recorded in model_performance table: model_id, task_type, outcome (success/failure/partial), quality_score, latency_ms, tokens_used, cost_usd

6. **Given** Performance is recorded
   **When** Scores are updated
   **Then** Model performance scores are updated based on outcomes (RL feedback loop)

7. **Given** Performance data accumulates
   **When** Routing decisions change
   **Then** Routing rules may be adjusted automatically based on learned performance

## Tasks / Subtasks

- [ ] Task 1: Create routing tables schema (AC: #1, #3, #5)
  - [ ] Add model_routing_rules table
  - [ ] Add routing_decisions table
  - [ ] Add model_performance table
  - [ ] Include all required columns

- [ ] Task 2: Define TaskType enum (AC: #1)
  - [ ] Create TaskType enum: CodeGeneration, CodeReview, Debugging, Planning, Documentation, Testing, Math, General
  - [ ] Implement task classification logic

- [ ] Task 3: Create ModelRegistry (AC: #2)
  - [ ] Define specialized models (Codestral, Qwen-Math)
  - [ ] Define generalist models (Claude, GPT-4o)
  - [ ] Include cost and quality metrics

- [ ] Task 4: Implement ModelRouter (AC: #1, #2, #3, #4)
  - [ ] Create ModelRouter service
  - [ ] Implement select_model() method
  - [ ] Record routing decisions
  - [ ] Apply routing preference (Quality, CostEfficient, Balanced, Latency)

- [ ] Task 5: Implement performance recording (AC: #5, #6)
  - [ ] Create record_performance() method
  - [ ] Track outcome, quality, latency, tokens, cost
  - [ ] Update model scores based on outcomes

- [ ] Task 6: Implement RL feedback loop (AC: #6, #7)
  - [ ] Calculate success rates per model per task type
  - [ ] Adjust specialized vs generalist preference
  - [ ] Update routing rules based on learned performance

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Task type classification
- Specialized vs generalist model routing
- RL-optimized selection based on performance history

**Model Routing Requirements:**
- RL-optimized selection between specialized and generalist models
- Performance tracking (quality, latency, cost, success rate)

### Task Types and Specialized Models

```rust
pub enum TaskType {
    CodeGeneration,  // Codestral
    CodeReview,
    Debugging,
    Planning,
    Documentation,
    Testing,
    Math,            // Qwen-Math
    General,
}

pub struct ModelRegistry {
    pub specialized: HashMap<TaskType, Vec<ModelConfig>>,
    pub generalist: Vec<ModelConfig>,
}
```

### Routing Preferences

```rust
pub enum RoutingPreference {
    Quality,        // Maximize quality, ignore cost
    CostEfficient,  // Minimize cost while meeting quality threshold
    Balanced,       // Balance quality and cost (score = quality / cost)
    Latency,        // Minimize response time
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/routing/task_type.rs`
- `crates/demiarch-core/src/domain/routing/model_registry.rs`
- `crates/demiarch-core/src/domain/routing/router.rs`
- `crates/demiarch-core/src/infrastructure/db/routing.rs`

**Database Schema:**
```sql
CREATE TABLE model_routing_rules (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    model_id TEXT NOT NULL,
    priority INTEGER NOT NULL,
    is_specialized INTEGER DEFAULT 0,
    min_quality_score REAL,
    max_cost_per_1k REAL,
    is_enabled INTEGER DEFAULT 1
);

CREATE TABLE routing_decisions (
    id TEXT PRIMARY KEY,
    agent_execution_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    selected_model TEXT NOT NULL,
    selection_reason TEXT,
    alternatives_considered TEXT,  -- JSON array
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE model_performance (
    id TEXT PRIMARY KEY,
    model_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    quality_score REAL,
    latency_ms INTEGER,
    tokens_used INTEGER,
    cost_usd REAL,
    recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Testing Requirements

- Task classification tests
- Model selection tests
- Performance recording tests
- RL feedback loop tests

### References

- [Source: docs/PRD.md#Dynamic-Model-Routing] - Model routing requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Model-Routing] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
