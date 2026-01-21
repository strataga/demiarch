# Story 7.1: Install Built-in Framework Plugins

Status: ready-for-dev

## Story

As a user,
I want Demiarch to include built-in plugins for major frameworks (Next.js, React Native, Flutter, etc.),
so that I can generate code for my preferred framework without manual setup.

## Acceptance Criteria

1. **Given** User selects a framework during project creation or generation
   **When** Framework is a built-in plugin (Next.js, Vue 3, Flutter, etc.)
   **Then** Plugin is loaded from installed_plugins table with source='builtin'

2. **Given** Plugin is loaded
   **When** Plugin metadata is displayed
   **Then** Plugin metadata is displayed: name, version, capabilities, api_version

3. **Given** Plugin is active
   **When** Code generation is triggered
   **Then** Plugin can generate components, API routes, and models for that framework

4. **Given** Plugin is used
   **When** User sees status
   **Then** User sees: "Using built-in plugin: Next.js v14"

## Tasks / Subtasks

- [ ] Task 1: Create installed_plugins table schema (AC: #1, #2)
  - [ ] Add migration for installed_plugins table
  - [ ] Include columns: id, name, version, source, source_url, capabilities, api_version, checksum, pricing_model, is_enabled
  - [ ] Add indexes for source and name queries

- [ ] Task 2: Define built-in plugin registry (AC: #1)
  - [ ] Create list of built-in frameworks
  - [ ] Include: Next.js, Vue 3, React Native, Flutter, Astro, SvelteKit
  - [ ] Define capabilities for each

- [ ] Task 3: Implement PluginLoader service (AC: #1, #2)
  - [ ] Create PluginLoader in demiarch-core
  - [ ] Load built-in plugins from embedded resources
  - [ ] Insert records with source='builtin'

- [ ] Task 4: Implement framework code generation (AC: #3)
  - [ ] Define plugin interface for code generation
  - [ ] Implement component generation
  - [ ] Implement API route generation
  - [ ] Implement model generation

- [ ] Task 5: Implement plugin status display (AC: #4)
  - [ ] Show active plugin in UI
  - [ ] Display version information
  - [ ] List available capabilities

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Built-in plugins loaded from embedded resources
- Plugin interface for code generation
- Three-tier trust model (built-in = fully trusted)

**Plugin Requirements:**
- Built-in plugins are signed and trusted
- No sandbox required for built-in plugins
- Version tracking for updates

### Built-in Frameworks

```rust
pub const BUILTIN_PLUGINS: &[BuiltinPlugin] = &[
    BuiltinPlugin {
        name: "nextjs",
        display_name: "Next.js",
        version: "14.0.0",
        capabilities: &["component", "api_route", "model", "page"],
    },
    BuiltinPlugin {
        name: "vue3",
        display_name: "Vue 3",
        version: "3.4.0",
        capabilities: &["component", "composable", "store", "page"],
    },
    BuiltinPlugin {
        name: "react-native",
        display_name: "React Native",
        version: "0.73.0",
        capabilities: &["component", "screen", "navigation", "native_module"],
    },
    BuiltinPlugin {
        name: "flutter",
        display_name: "Flutter",
        version: "3.16.0",
        capabilities: &["widget", "screen", "provider", "model"],
    },
];
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/plugins/plugin.rs`
- `crates/demiarch-core/src/domain/plugins/loader.rs`
- `crates/demiarch-core/src/domain/plugins/builtin/mod.rs`
- `crates/demiarch-core/src/infrastructure/db/installed_plugins.rs`

**Database Schema:**
```sql
CREATE TABLE installed_plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    source TEXT NOT NULL,  -- 'builtin', 'local', 'registry'
    source_url TEXT,
    capabilities TEXT NOT NULL,  -- JSON array
    api_version TEXT NOT NULL,
    checksum TEXT,
    pricing_model TEXT DEFAULT 'free',
    is_enabled INTEGER DEFAULT 1,
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_installed_plugins_name ON installed_plugins(name);
CREATE INDEX idx_installed_plugins_source ON installed_plugins(source);
```

### Testing Requirements

- Built-in plugin loading tests
- Plugin capability tests
- Code generation tests per framework
- Plugin metadata display tests

### References

- [Source: docs/PRD.md#Plugin-System] - Plugin requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
