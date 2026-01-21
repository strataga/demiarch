# Story 7.7: Lifecycle Hooks for Plugin Events

Status: ready-for-dev

## Story

As a plugin developer or power user,
I want plugins to register handlers for lifecycle events (session_start, pre_generation, post_generation, on_error, on_checkpoint),
so that plugins can react to Demiarch events.

## Acceptance Criteria

1. **Given** Plugin is loaded and configured
   **When** Demiarch triggers lifecycle event (e.g., pre_generation)
   **Then** System queries lifecycle_hooks table for handlers with hook_type matching event

2. **Given** Handlers are found
   **When** Handlers are executed
   **Then** Each handler is executed based on handler_type: 'internal', 'plugin', 'script'

3. **Given** handler_type is 'plugin'
   **When** Handler is executed
   **Then** Handler config JSON includes plugin_id and invokes plugin function

4. **Given** handler_type is 'script'
   **When** Handler is executed
   **Then** Handler config JSON includes script_path and executes external script

5. **Given** Handler completes
   **When** Execution is recorded
   **Then** Record is added to hook_executions table with hook_id, session_id, trigger_context, result, duration_ms

6. **Given** Handler fails
   **When** Error is logged
   **Then** Error is logged and event continues

7. **Given** Multiple handlers exist
   **When** Priority is checked
   **Then** Handler priority is respected (lower number = higher priority)

## Tasks / Subtasks

- [ ] Task 1: Create lifecycle_hooks table schema (AC: #1)
  - [ ] Add migration for lifecycle_hooks table
  - [ ] Include columns: id, hook_type, handler_type, handler_config, priority, is_enabled
  - [ ] Add indexes for hook_type queries

- [ ] Task 2: Create hook_executions table schema (AC: #5)
  - [ ] Add migration for hook_executions table
  - [ ] Include columns: id, hook_id, session_id, trigger_context, result, duration_ms, executed_at
  - [ ] Add foreign keys

- [ ] Task 3: Implement HookManager service (AC: #1, #7)
  - [ ] Create HookManager in demiarch-core
  - [ ] Query hooks by hook_type
  - [ ] Order by priority ascending
  - [ ] Execute handlers in order

- [ ] Task 4: Implement handler types (AC: #2, #3, #4)
  - [ ] Implement internal handler execution
  - [ ] Implement plugin handler execution (via WASM)
  - [ ] Implement script handler execution

- [ ] Task 5: Implement execution logging (AC: #5, #6)
  - [ ] Record execution start/end
  - [ ] Track duration in milliseconds
  - [ ] Log errors and continue

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Lifecycle hooks for extensibility
- Multiple handler types (internal, plugin, script)
- Priority-based execution order

**Hook Types:**
- session_start: When user starts Demiarch
- session_end: When user ends session
- pre_generation: Before code generation starts
- post_generation: After code generation completes
- on_error: When an error occurs
- on_checkpoint: When checkpoint is created

### Hook System Design

```rust
pub enum HookType {
    SessionStart,
    SessionEnd,
    PreGeneration,
    PostGeneration,
    OnError,
    OnCheckpoint,
}

pub enum HandlerType {
    Internal,  // Built-in Demiarch handlers
    Plugin,    // WASM plugin handlers
    Script,    // External script handlers
}

pub struct LifecycleHook {
    pub id: String,
    pub hook_type: HookType,
    pub handler_type: HandlerType,
    pub handler_config: serde_json::Value,
    pub priority: i32,  // Lower = higher priority
    pub is_enabled: bool,
}

pub struct HookExecution {
    pub id: String,
    pub hook_id: String,
    pub session_id: String,
    pub trigger_context: serde_json::Value,
    pub result: serde_json::Value,
    pub duration_ms: u64,
    pub executed_at: DateTime<Utc>,
}

impl HookManager {
    pub async fn trigger(&self, hook_type: HookType, context: &HookContext) -> Result<()> {
        let hooks = self.db.query::<LifecycleHook>(
            "SELECT * FROM lifecycle_hooks
             WHERE hook_type = ? AND is_enabled = 1
             ORDER BY priority ASC",
            hook_type
        ).await?;

        for hook in hooks {
            let start = Instant::now();
            let result = match hook.handler_type {
                HandlerType::Internal => self.execute_internal(&hook, context).await,
                HandlerType::Plugin => self.execute_plugin(&hook, context).await,
                HandlerType::Script => self.execute_script(&hook, context).await,
            };

            // Record execution
            let execution = HookExecution {
                id: Uuid::new_v4().to_string(),
                hook_id: hook.id,
                session_id: context.session_id.clone(),
                trigger_context: serde_json::to_value(context)?,
                result: result.unwrap_or_else(|e| json!({ "error": e.to_string() })),
                duration_ms: start.elapsed().as_millis() as u64,
                executed_at: Utc::now(),
            };
            self.db.insert(&execution).await?;

            // Log errors but continue
            if let Err(e) = result {
                warn!("Hook {} failed: {}", hook.id, e);
            }
        }

        Ok(())
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/hooks/hook.rs`
- `crates/demiarch-core/src/domain/hooks/manager.rs`
- `crates/demiarch-core/src/domain/hooks/handlers/internal.rs`
- `crates/demiarch-core/src/domain/hooks/handlers/plugin.rs`
- `crates/demiarch-core/src/domain/hooks/handlers/script.rs`
- `crates/demiarch-core/src/infrastructure/db/lifecycle_hooks.rs`

**Database Schema:**
```sql
CREATE TABLE lifecycle_hooks (
    id TEXT PRIMARY KEY,
    hook_type TEXT NOT NULL,  -- 'session_start', 'pre_generation', etc.
    handler_type TEXT NOT NULL,  -- 'internal', 'plugin', 'script'
    handler_config TEXT NOT NULL,  -- JSON config
    priority INTEGER DEFAULT 100,
    is_enabled INTEGER DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_lifecycle_hooks_type ON lifecycle_hooks(hook_type, is_enabled);

CREATE TABLE hook_executions (
    id TEXT PRIMARY KEY,
    hook_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    trigger_context TEXT NOT NULL,  -- JSON
    result TEXT,  -- JSON
    duration_ms INTEGER NOT NULL,
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (hook_id) REFERENCES lifecycle_hooks(id)
);

CREATE INDEX idx_hook_executions_hook ON hook_executions(hook_id);
CREATE INDEX idx_hook_executions_session ON hook_executions(session_id);
```

### Testing Requirements

- Hook triggering tests
- Handler type execution tests
- Priority ordering tests
- Error handling tests
- Execution logging tests

### References

- [Source: docs/PRD.md#Lifecycle-Hooks] - Hook requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.7] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
