# Story 7.2: Load and Execute WASM Plugins

Status: ready-for-dev

## Story

As a user,
I want to install and use third-party plugins compiled to WASM,
so that I can extend Demiarch's capabilities securely.

## Acceptance Criteria

1. **Given** User has a WASM plugin file and installs it
   **When** Plugin is installed
   **Then** installed_plugins record is created with source='local' or 'registry', source_url, checksum

2. **Given** Plugin is loaded
   **When** Sandbox is configured
   **Then** Plugin is loaded into Wasmtime sandbox with configured capabilities (read_files, write_files, network, max_memory_mb, max_cpu_seconds)

3. **Given** Sandbox is running
   **When** Fuel limit is enforced
   **Then** Fuel limit is enforced (10M fuel max, randomized to prevent side-channel attacks)

4. **Given** Plugin executes
   **When** Capabilities are checked
   **Then** Only allowed capabilities are linked (plugin_func_wrap for read, write)

5. **Given** Plugin completes
   **When** Metrics are recorded
   **Then** Plugin execution is logged with fuel consumption metrics

6. **Given** Plugin exceeds limits or crashes
   **When** Termination occurs
   **Then** Execution is terminated and error is returned to user

7. **Given** Anomaly is detected
   **When** Security review is needed
   **Then** Fuel consumption outliers are flagged for security review

## Tasks / Subtasks

- [ ] Task 1: Implement WASM plugin installer (AC: #1)
  - [ ] Create plugin installation flow
  - [ ] Calculate and store checksum (SHA-256)
  - [ ] Create installed_plugins record
  - [ ] Support local files and registry URLs

- [ ] Task 2: Implement Wasmtime sandbox (AC: #2, #3)
  - [ ] Initialize Wasmtime engine
  - [ ] Configure memory limits (max_memory_mb)
  - [ ] Configure CPU limits (fuel)
  - [ ] Randomize fuel limit for security

- [ ] Task 3: Implement capability linking (AC: #4)
  - [ ] Define host functions for read_files
  - [ ] Define host functions for write_files
  - [ ] Define host functions for network (if allowed)
  - [ ] Only link allowed capabilities

- [ ] Task 4: Implement execution logging (AC: #5)
  - [ ] Track fuel consumption
  - [ ] Log execution start/end
  - [ ] Record metrics to plugin_executions table

- [ ] Task 5: Implement limit enforcement (AC: #6, #7)
  - [ ] Catch fuel exhaustion
  - [ ] Handle crashes gracefully
  - [ ] Detect and flag outliers
  - [ ] Add to audit_log for security review

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- WASM sandboxing with Wasmtime
- Capability-based security
- Fuel-based resource limiting

**Security Requirements:**
- Randomized fuel limits prevent timing attacks
- Only requested capabilities are linked
- Outlier detection for security review

### WASM Sandbox Configuration

```rust
pub struct WasmSandbox {
    engine: Engine,
    store: Store<PluginState>,
    instance: Instance,
}

pub struct SandboxConfig {
    pub max_memory_mb: u32,        // Default: 256
    pub max_fuel: u64,             // Default: 10_000_000
    pub fuel_randomization: bool,  // Default: true
    pub capabilities: Capabilities,
}

pub struct Capabilities {
    pub read_files: bool,
    pub write_files: bool,
    pub network: bool,
    pub execute_commands: bool,  // Dangerous, rarely granted
}

impl WasmSandbox {
    pub fn new(wasm_bytes: &[u8], config: SandboxConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        engine_config.consume_fuel(true);

        let engine = Engine::new(&engine_config)?;
        let mut store = Store::new(&engine, PluginState::default());

        // Randomize fuel to prevent side-channel attacks
        let fuel = if config.fuel_randomization {
            let variance = config.max_fuel / 10;  // 10% variance
            config.max_fuel - rand::random::<u64>() % variance
        } else {
            config.max_fuel
        };
        store.add_fuel(fuel)?;

        // Link only allowed capabilities
        let mut linker = Linker::new(&engine);
        if config.capabilities.read_files {
            linker.func_wrap("host", "read_file", host_read_file)?;
        }
        if config.capabilities.write_files {
            linker.func_wrap("host", "write_file", host_write_file)?;
        }

        let module = Module::new(&engine, wasm_bytes)?;
        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Self { engine, store, instance })
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/plugins/wasm/sandbox.rs`
- `crates/demiarch-core/src/domain/plugins/wasm/host_functions.rs`
- `crates/demiarch-core/src/domain/plugins/wasm/executor.rs`
- `crates/demiarch-core/src/infrastructure/db/plugin_executions.rs`

**Database Schema:**
```sql
CREATE TABLE plugin_executions (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    status TEXT NOT NULL,  -- 'running', 'completed', 'failed', 'terminated'
    fuel_consumed INTEGER,
    memory_peak_mb INTEGER,
    error_message TEXT,
    FOREIGN KEY (plugin_id) REFERENCES installed_plugins(id)
);

CREATE INDEX idx_plugin_executions_plugin ON plugin_executions(plugin_id);
CREATE INDEX idx_plugin_executions_status ON plugin_executions(status);
```

### Testing Requirements

- WASM loading tests
- Sandbox capability tests
- Fuel limit enforcement tests
- Crash handling tests
- Outlier detection tests

### References

- [Source: docs/PRD.md#Plugin-System] - Plugin requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - Wasmtime details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
