# Story 7.5: Configure Plugin Capabilities and Permissions

Status: ready-for-dev

## Story

As a user,
I want to review and approve plugin capabilities before first use,
so that I understand what a plugin can access.

## Acceptance Criteria

1. **Given** User installs a plugin with capabilities configured
   **When** User first uses the plugin
   **Then** Modal displays requested capabilities: read_files (yes/no), write_files (yes/no), network (yes/no), execute_commands (yes/no)

2. **Given** Capabilities are displayed
   **When** User reviews them
   **Then** Each capability shows plain-language explanation (e.g., "read_files: This plugin can read your project files")

3. **Given** User reviews capabilities
   **When** User makes selection
   **Then** User can approve all capabilities or select specific ones

4. **Given** Capabilities are denied
   **When** Sandbox is configured
   **Then** Denied capabilities are not linked to WASM sandbox

5. **Given** User's choice is made
   **When** Choice is saved
   **Then** User's choice is saved in plugin_config table per capability

6. **Given** User denies all capabilities
   **When** Plugin is disabled
   **Then** Plugin is disabled with message: "Plugin requires permissions to function"

## Tasks / Subtasks

- [ ] Task 1: Create plugin_config table schema (AC: #5)
  - [ ] Add migration for plugin_config table
  - [ ] Include columns: id, plugin_id, capability, is_granted, granted_at
  - [ ] Add foreign key to installed_plugins

- [ ] Task 2: Implement capability modal (AC: #1, #2)
  - [ ] Create CapabilityReviewModal component
  - [ ] List all requested capabilities
  - [ ] Show plain-language explanations

- [ ] Task 3: Implement capability selection (AC: #3)
  - [ ] Add "Approve All" button
  - [ ] Add individual toggles per capability
  - [ ] Track user selections

- [ ] Task 4: Integrate with sandbox (AC: #4)
  - [ ] Read granted capabilities from plugin_config
  - [ ] Only link granted capabilities
  - [ ] Handle partial permissions

- [ ] Task 5: Implement minimum capability check (AC: #6)
  - [ ] Define required capabilities per plugin
  - [ ] Check if minimum requirements met
  - [ ] Disable plugin if requirements not met
  - [ ] Show explanatory message

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Capability-based permission model
- User approval before first use
- Granular permission control

**Security Requirements:**
- Capabilities denied by default
- Clear explanation of each capability
- Audit trail for permission grants

### Capability Definitions

```rust
pub enum Capability {
    ReadFiles,       // Read project files
    WriteFiles,      // Create/modify project files
    Network,         // Make network requests
    ExecuteCommands, // Run shell commands (dangerous)
}

impl Capability {
    pub fn explanation(&self) -> &'static str {
        match self {
            Capability::ReadFiles => "This plugin can read your project files to analyze code and generate suggestions.",
            Capability::WriteFiles => "This plugin can create and modify files in your project.",
            Capability::Network => "This plugin can make network requests to external services.",
            Capability::ExecuteCommands => "This plugin can run shell commands on your system. This is a high-risk permission.",
        }
    }

    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Capability::ReadFiles => RiskLevel::Low,
            Capability::WriteFiles => RiskLevel::Medium,
            Capability::Network => RiskLevel::Medium,
            Capability::ExecuteCommands => RiskLevel::High,
        }
    }
}

pub struct CapabilityGrant {
    pub plugin_id: String,
    pub capability: Capability,
    pub is_granted: bool,
    pub granted_at: Option<DateTime<Utc>>,
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/plugins/capabilities.rs`
- `crates/demiarch-core/src/infrastructure/db/plugin_config.rs`
- `crates/demiarch-gui/src/components/plugins/CapabilityReviewModal.tsx`

**Database Schema:**
```sql
CREATE TABLE plugin_config (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    capability TEXT NOT NULL,
    is_granted INTEGER DEFAULT 0,
    granted_at DATETIME,
    FOREIGN KEY (plugin_id) REFERENCES installed_plugins(id)
);

CREATE UNIQUE INDEX idx_plugin_config_unique ON plugin_config(plugin_id, capability);
```

### UI Components

```tsx
// CapabilityReviewModal.tsx
interface CapabilityReviewModalProps {
  plugin: Plugin;
  onApprove: (grants: CapabilityGrant[]) => void;
  onCancel: () => void;
}

const CAPABILITY_INFO = {
  read_files: {
    icon: Eye,
    title: "Read Files",
    description: "This plugin can read your project files to analyze code and generate suggestions.",
    risk: "low",
  },
  write_files: {
    icon: Edit,
    title: "Write Files",
    description: "This plugin can create and modify files in your project.",
    risk: "medium",
  },
  network: {
    icon: Wifi,
    title: "Network Access",
    description: "This plugin can make network requests to external services.",
    risk: "medium",
  },
  execute_commands: {
    icon: Terminal,
    title: "Execute Commands",
    description: "This plugin can run shell commands on your system.",
    risk: "high",
  },
};
```

### Testing Requirements

- Capability display tests
- Permission grant/deny tests
- Sandbox integration tests
- Minimum capability check tests

### References

- [Source: docs/PRD.md#Plugin-System] - Capability requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
