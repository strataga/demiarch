# Story 1.6: Track Basic LLM Costs

Status: ready-for-dev

## Story

As a user,
I want to see how much I'm spending on LLM calls,
so that I can manage my AI development budget.

## Acceptance Criteria

1. **Given** User has configured API key and has made at least one LLM request
   **When** User runs `demiarch costs --project my-project`
   **Then** System displays total cost spent for project in USD

2. **Given** Cost command is executed
   **When** Results are displayed
   **Then** Cost breakdown shows tokens used per model (prompt + completion)

3. **Given** LLM usage data exists
   **When** Costs are calculated
   **Then** Costs are retrieved from llm_usage table with accurate calculations

4. **Given** GUI is available
   **When** User views costs in GUI
   **Then** A dashboard panel shows daily cost vs budget limit

5. **Given** No LLM usage exists
   **When** Costs are displayed
   **Then** Zero cost displays as "$0.00" not empty or null

## Tasks / Subtasks

- [ ] Task 1: Create llm_usage table and schema (AC: #3)
  - [ ] Add migration for llm_usage table
  - [ ] Include columns: id, project_id, agent_execution_id, model, prompt_tokens, completion_tokens, cached_tokens, cost_usd, request_timestamp
  - [ ] Add indexes for project_id and request_timestamp queries

- [ ] Task 2: Implement CostTracker domain service (AC: #3)
  - [ ] Create CostTracker struct with pricing HashMap
  - [ ] Add model pricing data (Claude, GPT-4o, Haiku, etc.)
  - [ ] Implement calculate_cost() method
  - [ ] Implement record_usage() method

- [ ] Task 3: Implement cost CLI command (AC: #1, #2, #5)
  - [ ] Create `demiarch costs` command
  - [ ] Add --project flag for project filtering
  - [ ] Format output with total cost, per-model breakdown
  - [ ] Handle zero cost display properly

- [ ] Task 4: Create cost dashboard data endpoint (AC: #4)
  - [ ] Create Tauri command for cost data retrieval
  - [ ] Include daily spending, budget limit, breakdown by model
  - [ ] Return structured JSON for frontend

- [ ] Task 5: Integrate cost tracking with LLM client
  - [ ] Update LLM client to call CostTracker after each request
  - [ ] Extract token counts from API response
  - [ ] Calculate and persist cost

- [ ] Task 6: Add GUI cost panel placeholder
  - [ ] Create CostPanel React component
  - [ ] Display basic cost information
  - [ ] Show daily vs budget visualization

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain service pattern for CostTracker
- Repository pattern for llm_usage table access
- CLI-as-Library: cost commands usable by CLI and GUI

**Cost Tracking Requirements:**
- Real-time cost tracking per model, per feature, per project
- Token-efficient calculations (prompt + completion + cached)
- Budget enforcement with daily limits (to be added in Epic 8)

### Model Pricing Reference

```rust
// Prices as of 2025 (update periodically)
"anthropic/claude-sonnet-4-20250514" => { prompt: 0.003, completion: 0.015, cached: 0.0003 }
"anthropic/claude-3-5-haiku-latest" => { prompt: 0.0008, completion: 0.004, cached: 0.00008 }
"openai/gpt-4o" => { prompt: 0.005, completion: 0.015, cached: 0.0025 }
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/cost/` - Cost domain module
- `crates/demiarch-core/src/infrastructure/db/llm_usage.rs` - Repository
- `crates/demiarch-cli/src/commands/costs.rs` - CLI command
- `crates/demiarch-gui/src/components/CostPanel.tsx` - GUI component

**Database Schema:**
```sql
CREATE TABLE llm_usage (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    agent_execution_id TEXT,
    model TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    cached_tokens INTEGER DEFAULT 0,
    cost_usd REAL NOT NULL,
    request_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

### Testing Requirements

- Unit tests for cost calculation accuracy
- Integration tests for usage recording
- CLI output formatting tests
- Edge case tests (zero cost, null values)

### References

- [Source: docs/PRD.md#Cost-Management] - Cost tracking requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Cost-Management] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.6] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
