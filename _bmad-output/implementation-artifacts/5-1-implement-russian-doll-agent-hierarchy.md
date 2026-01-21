# Story 5.1: Implement Russian Doll Agent Hierarchy

Status: ready-for-dev

## Story

As a user,
I want Demiarch to use a 3-level agent hierarchy (Orchestrator → Planner → Coder/Reviewer/Tester) for code generation,
so that complex features are broken down and delegated intelligently.

## Acceptance Criteria

1. **Given** User triggers code generation for a complex feature
   **When** Orchestrator agent (Level 1) receives request
   **Then** Orchestrator spawns a Planner agent (Level 2) via AgentTool call

2. **Given** Planner receives task
   **When** Task is analyzed
   **Then** Planner decomposes feature into tasks (e.g., "Create UI components", "Implement API routes")

3. **Given** Feature is decomposed
   **When** Worker agents are needed
   **Then** Planner spawns Coder, Reviewer, and Tester agents (Level 3) for each task

4. **Given** Agent executions occur
   **When** Executions are tracked
   **Then** All agent executions are recorded in agent_executions table with parent_agent_id, agent_type, status

5. **Given** Coder agent completes code generation
   **When** Results are returned
   **Then** Result is returned to Planner

6. **Given** All Level 3 agents complete
   **When** Results bubble up
   **Then** Reviewer validates and Tester creates tests, all results bubble up through hierarchy to Orchestrator

7. **Given** Generation is in progress
   **When** User views status
   **Then** User sees: "Orchestrator → Planner → Coder, Reviewer, Tester" status

8. **Given** All tasks complete
   **When** Generation finishes
   **Then** Generation completes with all tasks finished

## Tasks / Subtasks

- [ ] Task 1: Create agent_executions table schema (AC: #4)
  - [ ] Add migration for agent_executions table
  - [ ] Include columns: id, parent_agent_id, project_id, feature_id, agent_type, status, input_context, output_result, error_message, tokens_used, cost_usd, started_at, completed_at
  - [ ] Add indexes for hierarchy queries

- [ ] Task 2: Define Agent types and structures (AC: #1, #3)
  - [ ] Create AgentType enum: Orchestrator, Planner, Coder, Reviewer, Tester
  - [ ] Create Agent struct with hierarchy support
  - [ ] Implement parent-child relationships

- [ ] Task 3: Implement Orchestrator agent (AC: #1)
  - [ ] Create OrchestratorAgent struct
  - [ ] Implement feature context building
  - [ ] Implement Planner spawning via AgentTool
  - [ ] Track child agent results

- [ ] Task 4: Implement Planner agent (AC: #2, #3)
  - [ ] Create PlannerAgent struct
  - [ ] Implement task decomposition logic
  - [ ] Spawn appropriate worker agents
  - [ ] Coordinate worker execution

- [ ] Task 5: Implement Worker agents (AC: #5, #6)
  - [ ] Create CoderAgent for code generation
  - [ ] Create ReviewerAgent for code validation
  - [ ] Create TesterAgent for test generation
  - [ ] Implement result return to parent

- [ ] Task 6: Implement execution tracking (AC: #4, #7)
  - [ ] Record all agent executions
  - [ ] Track tokens and costs per agent
  - [ ] Provide status for UI display

- [ ] Task 7: Implement result aggregation (AC: #8)
  - [ ] Bubble results up through hierarchy
  - [ ] Aggregate final results at Orchestrator
  - [ ] Handle failures at any level

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Russian Doll agent hierarchy with 3 levels
- Each agent runs in separate Tokio task
- Parent-child relationships tracked in database

**Agent Hierarchy:**
```
Level 1: Orchestrator (teal)
    ↓
Level 2: Planner (magenta)
    ↓
Level 3: Coder, Reviewer, Tester (amber)
```

### Agent Structure

```rust
pub enum AgentType {
    Orchestrator,  // Level 1
    Planner,       // Level 2
    Coder,         // Level 3
    Reviewer,      // Level 3
    Tester,        // Level 3
}

pub struct Agent {
    pub id: String,
    pub parent_id: Option<String>,
    pub agent_type: AgentType,
    pub project_id: String,
    pub feature_id: String,
    pub context: AgentContext,
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/agents/` - Agent domain
- `crates/demiarch-core/src/domain/agents/orchestrator.rs`
- `crates/demiarch-core/src/domain/agents/planner.rs`
- `crates/demiarch-core/src/domain/agents/workers/` - Coder, Reviewer, Tester
- `crates/demiarch-core/src/infrastructure/db/agent_executions.rs`

**Database Schema:**
```sql
CREATE TABLE agent_executions (
    id TEXT PRIMARY KEY,
    parent_agent_id TEXT,
    project_id TEXT NOT NULL,
    feature_id TEXT,
    agent_type TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    input_context TEXT,
    output_result TEXT,
    error_message TEXT,
    tokens_used INTEGER,
    cost_usd REAL,
    started_at DATETIME,
    completed_at DATETIME,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (parent_agent_id) REFERENCES agent_executions(id)
);
```

### Testing Requirements

- Unit tests for each agent type
- Hierarchy traversal tests
- Integration tests for full generation flow
- Failure handling tests

### References

- [Source: docs/PRD.md#Russian-Doll-Agent-System] - Agent hierarchy requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Agent-System-Resilience] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
