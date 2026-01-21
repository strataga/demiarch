# Story 7.6: Implement Plugin Pricing Models

Status: ready-for-dev

## Story

As a user,
I want Demiarch to support multiple plugin pricing models (free, paid, usage-based, trial, watermark),
so that I can choose plugins with my preferred pricing.

## Acceptance Criteria

1. **Given** User installs a plugin with pricing_model specified
   **When** Plugin pricing model is loaded
   **Then** installed_plugins.pricing_model is set to: 'free', 'paid', 'usage', 'trial', or 'watermark'

2. **Given** pricing_model is 'paid'
   **When** User installs plugin
   **Then** User is prompted to enter license_key during installation

3. **Given** pricing_model is 'usage'
   **When** Plugin is used
   **Then** plugin_usage records are tracked per plugin with usage_count, period_start, period_end

4. **Given** Usage limit is exceeded
   **When** User tries to use plugin
   **Then** User sees: "Usage limit exceeded. Upgrade to continue."

5. **Given** pricing_model is 'trial'
   **When** User views plugin
   **Then** User sees trial days remaining and expiration date

6. **Given** pricing_model is 'watermark'
   **When** Code is generated
   **Then** Generated code includes visible watermark: "Created with {PluginName}"

7. **Given** Watermark is present
   **When** User wants to remove it
   **Then** Watermark removal prompts: "Remove watermark by purchasing full license"

## Tasks / Subtasks

- [ ] Task 1: Create plugin_usage table schema (AC: #3)
  - [ ] Add migration for plugin_usage table
  - [ ] Include columns: id, plugin_id, usage_count, free_quota, period_start, period_end
  - [ ] Add foreign key to installed_plugins

- [ ] Task 2: Implement pricing model handling (AC: #1, #2)
  - [ ] Define PricingModel enum
  - [ ] Handle each model during installation
  - [ ] Prompt for license key when needed

- [ ] Task 3: Implement usage tracking (AC: #3, #4)
  - [ ] Track usage per plugin
  - [ ] Enforce free quota limits
  - [ ] Show usage exceeded message

- [ ] Task 4: Implement trial management (AC: #5)
  - [ ] Track trial start date
  - [ ] Calculate days remaining
  - [ ] Display trial status

- [ ] Task 5: Implement watermark injection (AC: #6, #7)
  - [ ] Add watermark to generated code
  - [ ] Make watermark visible and consistent
  - [ ] Show upgrade prompt for removal

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Multiple pricing model support
- Usage tracking for metered billing
- Watermark injection for freemium

**Business Requirements:**
- Clear pricing model indicators
- Graceful degradation when limits hit
- Easy upgrade path

### Pricing Models

```rust
pub enum PricingModel {
    Free,      // No cost, all features
    Paid,      // Requires license key
    Usage,     // Free quota, then pay per use
    Trial,     // Full features for limited time
    Watermark, // Free with visible branding
}

pub struct PluginUsage {
    pub id: String,
    pub plugin_id: String,
    pub usage_count: u32,
    pub free_quota: u32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl PluginUsage {
    pub fn is_quota_exceeded(&self) -> bool {
        self.usage_count >= self.free_quota
    }

    pub fn remaining(&self) -> u32 {
        self.free_quota.saturating_sub(self.usage_count)
    }
}

pub struct WatermarkConfig {
    pub plugin_name: String,
    pub template: String,  // e.g., "/* Created with {plugin_name} - https://example.com */"
}

impl WatermarkConfig {
    pub fn inject(&self, code: &str) -> String {
        let watermark = self.template.replace("{plugin_name}", &self.plugin_name);
        format!("{}\n\n{}", watermark, code)
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/plugins/pricing.rs`
- `crates/demiarch-core/src/domain/plugins/usage.rs`
- `crates/demiarch-core/src/domain/plugins/watermark.rs`
- `crates/demiarch-core/src/infrastructure/db/plugin_usage.rs`

**Database Schema:**
```sql
CREATE TABLE plugin_usage (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    usage_count INTEGER DEFAULT 0,
    free_quota INTEGER NOT NULL,
    period_start DATETIME NOT NULL,
    period_end DATETIME NOT NULL,
    FOREIGN KEY (plugin_id) REFERENCES installed_plugins(id)
);

CREATE UNIQUE INDEX idx_plugin_usage_plugin ON plugin_usage(plugin_id);
```

### UI Components

```tsx
// PricingBadge.tsx
interface PricingBadgeProps {
  model: 'free' | 'paid' | 'usage' | 'trial' | 'watermark';
  usage?: PluginUsage;
  trialEndsAt?: Date;
}

const PricingBadge: React.FC<PricingBadgeProps> = ({ model, usage, trialEndsAt }) => {
  switch (model) {
    case 'usage':
      return (
        <div className="text-sm">
          {usage?.remaining()} / {usage?.free_quota} uses remaining
        </div>
      );
    case 'trial':
      const days = Math.ceil((trialEndsAt - Date.now()) / (1000 * 60 * 60 * 24));
      return <div className="text-sm text-amber-400">{days} days left in trial</div>;
    // ...
  }
};
```

### Testing Requirements

- Pricing model handling tests
- Usage tracking tests
- Trial expiration tests
- Watermark injection tests

### References

- [Source: docs/PRD.md#Plugin-System] - Pricing requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.6] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
