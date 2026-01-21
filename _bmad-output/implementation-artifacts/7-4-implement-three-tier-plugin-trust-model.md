# Story 7.4: Implement Three-Tier Plugin Trust Model

Status: ready-for-dev

## Story

As a user,
I want to see plugin trust levels (built-in, verified third-party, unverified) with appropriate warnings,
so that I can make informed decisions about plugin safety.

## Acceptance Criteria

1. **Given** User views plugins in GUI or runs plugin list command
   **When** Plugins are displayed
   **Then** Each plugin shows trust badge:
     - "Built-in" (green checkmark) - fully trusted
     - "Verified" (blue shield) - signed with ed25519, third-party
     - "Unverified" (amber warning) - no signature, sandboxed only

2. **Given** Unverified plugin is displayed
   **When** Warning is shown
   **Then** Unverified plugins show warning: "This plugin is unverified. Use at your own risk."

3. **Given** Verified plugin is displayed
   **When** Publisher is shown
   **Then** Verified plugins show: "Signed by {publisher} verified at {date}"

4. **Given** Built-in plugin is displayed
   **When** Badge is shown
   **Then** Built-in plugins show: "Official Demiarch plugin"

5. **Given** User wants to filter
   **When** Filter is applied
   **Then** User can filter plugins by trust level

## Tasks / Subtasks

- [ ] Task 1: Define TrustLevel enum (AC: #1)
  - [ ] Create TrustLevel enum: BuiltIn, Verified, Unverified
  - [ ] Add trust_level column to installed_plugins
  - [ ] Determine trust level on installation

- [ ] Task 2: Implement trust badge display (AC: #1, #2, #3, #4)
  - [ ] Create TrustBadge component
  - [ ] Style each trust level distinctly
  - [ ] Show appropriate icon and color

- [ ] Task 3: Implement warning messages (AC: #2)
  - [ ] Show warning for unverified plugins
  - [ ] Require acknowledgment before first use
  - [ ] Log acknowledgment in audit_log

- [ ] Task 4: Implement publisher verification (AC: #3)
  - [ ] Extract publisher from signature
  - [ ] Display publisher name
  - [ ] Show verification date

- [ ] Task 5: Implement trust filter (AC: #5)
  - [ ] Add filter dropdown in plugin list
  - [ ] Filter by trust level
  - [ ] Persist filter preference

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Three-tier plugin trust model
- Visual trust indicators
- Acknowledgment for unverified plugins

**Security Requirements:**
- Clear visual distinction between trust levels
- Warnings for unverified plugins
- Audit trail for acknowledgments

### Trust Level Definitions

```rust
pub enum TrustLevel {
    BuiltIn,     // Shipped with Demiarch, fully trusted
    Verified,    // Third-party, signed with ed25519
    Unverified,  // No signature, sandboxed only
}

pub struct TrustBadge {
    pub level: TrustLevel,
    pub icon: &'static str,
    pub color: &'static str,
    pub label: &'static str,
}

impl TrustLevel {
    pub fn badge(&self) -> TrustBadge {
        match self {
            TrustLevel::BuiltIn => TrustBadge {
                level: TrustLevel::BuiltIn,
                icon: "check-circle",
                color: "#00f5d4",  // teal
                label: "Official Demiarch plugin",
            },
            TrustLevel::Verified => TrustBadge {
                level: TrustLevel::Verified,
                icon: "shield-check",
                color: "#3b82f6",  // blue
                label: "Verified third-party",
            },
            TrustLevel::Unverified => TrustBadge {
                level: TrustLevel::Unverified,
                icon: "alert-triangle",
                color: "#ffc300",  // amber
                label: "Unverified - Use at your own risk",
            },
        }
    }
}

pub fn determine_trust_level(plugin: &Plugin) -> TrustLevel {
    if plugin.source == "builtin" {
        TrustLevel::BuiltIn
    } else if plugin.signature.is_some() && verify_publisher_signature(plugin).is_ok() {
        TrustLevel::Verified
    } else {
        TrustLevel::Unverified
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/plugins/trust.rs`
- `crates/demiarch-gui/src/components/plugins/TrustBadge.tsx`
- `crates/demiarch-gui/src/components/plugins/PluginList.tsx`

**Database Schema:**
```sql
-- Add trust_level column to installed_plugins
ALTER TABLE installed_plugins ADD COLUMN trust_level TEXT DEFAULT 'unverified';
ALTER TABLE installed_plugins ADD COLUMN publisher TEXT;
ALTER TABLE installed_plugins ADD COLUMN signature TEXT;
ALTER TABLE installed_plugins ADD COLUMN verified_at DATETIME;
```

### UI Components

```tsx
// TrustBadge.tsx
interface TrustBadgeProps {
  level: 'builtin' | 'verified' | 'unverified';
  publisher?: string;
  verifiedAt?: Date;
}

const TrustBadge: React.FC<TrustBadgeProps> = ({ level, publisher, verifiedAt }) => {
  const config = {
    builtin: { icon: CheckCircle, color: 'text-teal-400', label: 'Official Demiarch plugin' },
    verified: { icon: ShieldCheck, color: 'text-blue-400', label: `Signed by ${publisher}` },
    unverified: { icon: AlertTriangle, color: 'text-amber-400', label: 'Unverified' },
  }[level];

  return (
    <div className={`flex items-center gap-2 ${config.color}`}>
      <config.icon className="w-4 h-4" />
      <span className="text-sm">{config.label}</span>
    </div>
  );
};
```

### Testing Requirements

- Trust level determination tests
- Badge display tests
- Warning acknowledgment tests
- Filter functionality tests

### References

- [Source: docs/PRD.md#Plugin-System] - Trust model requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
