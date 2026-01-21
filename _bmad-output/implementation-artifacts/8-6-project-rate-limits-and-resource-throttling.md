# Story 8.6: Project Rate Limits and Resource Throttling

Status: ready-for-dev

## Story

As a user,
I want Demiarch to enforce project-specific rate limits (hourly requests, concurrent agents),
so that resource usage stays controlled and predictable.

## Acceptance Criteria

1. **Given** User has configured rate limits or uses defaults
   **When** User initiates operations (LLM requests, file operations)
   **Then** System checks project_rate_limits table for limit_type (hourly_requests, concurrent_agents)

2. **Given** Limit is checked
   **When** Usage is tracked
   **Then** Current value is tracked and incremented

3. **Given** Usage is incremented
   **When** Limit is reached
   **Then** When current_value reaches limit_value, operation is throttled

4. **Given** Operation is throttled
   **When** User sees message
   **Then** Message shown: "Rate limit exceeded. Wait {reset_at} to continue."

5. **Given** Reset time passes
   **When** Counter resets
   **Then** current_value is reset to 0

6. **Given** User views project settings
   **When** Limits are displayed
   **Then** Rate limits show: type, current_value, limit_value, reset_at

7. **Given** User adjusts limits
   **When** Adjustment is made
   **Then** User can adjust limits (not below system minimums)

8. **Given** Limits are set
   **When** Persistence is checked
   **Then** Limits persist across sessions

## Tasks / Subtasks

- [ ] Task 1: Create project_rate_limits table schema (AC: #1, #6, #8)
  - [ ] Add migration for project_rate_limits table
  - [ ] Include columns: id, project_id, limit_type, limit_value, current_value, reset_at
  - [ ] Add indexes for project and type queries

- [ ] Task 2: Implement rate limit checking (AC: #1, #2)
  - [ ] Create RateLimiter service
  - [ ] Check limits before operations
  - [ ] Increment usage counters

- [ ] Task 3: Implement throttling (AC: #3, #4)
  - [ ] Reject operations when limit reached
  - [ ] Show throttle message with reset time
  - [ ] Calculate wait time

- [ ] Task 4: Implement counter reset (AC: #5)
  - [ ] Reset counters at scheduled times
  - [ ] Handle hourly reset for hourly_requests
  - [ ] Handle real-time tracking for concurrent_agents

- [ ] Task 5: Implement limit configuration UI (AC: #6, #7)
  - [ ] Show current limits in settings
  - [ ] Allow adjustment with validation
  - [ ] Enforce system minimums

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Per-project rate limiting
- Multiple limit types
- User-configurable limits with minimums

**Rate Limit Types:**
- hourly_requests: Max LLM requests per hour
- concurrent_agents: Max simultaneous agents
- tokens_per_minute: Max tokens used per minute

### Rate Limiter Design

```rust
pub enum LimitType {
    HourlyRequests,
    ConcurrentAgents,
    TokensPerMinute,
}

pub struct ProjectRateLimit {
    pub id: String,
    pub project_id: String,
    pub limit_type: LimitType,
    pub limit_value: u32,
    pub current_value: u32,
    pub reset_at: DateTime<Utc>,
}

pub struct RateLimiter {
    db: Database,
    system_minimums: HashMap<LimitType, u32>,
}

impl RateLimiter {
    pub async fn check_and_increment(&self, project_id: &str, limit_type: LimitType) -> Result<RateLimitResult> {
        let limit = self.get_or_create_limit(project_id, limit_type).await?;

        // Check if reset needed
        if Utc::now() >= limit.reset_at {
            self.reset_counter(project_id, limit_type).await?;
            return Ok(RateLimitResult::Allowed);
        }

        // Check if under limit
        if limit.current_value < limit.limit_value {
            self.increment_counter(project_id, limit_type).await?;
            return Ok(RateLimitResult::Allowed);
        }

        Ok(RateLimitResult::Throttled {
            reset_at: limit.reset_at,
            wait_seconds: (limit.reset_at - Utc::now()).num_seconds() as u32,
        })
    }

    pub async fn set_limit(&self, project_id: &str, limit_type: LimitType, value: u32) -> Result<()> {
        let minimum = self.system_minimums.get(&limit_type).copied().unwrap_or(1);
        let clamped_value = value.max(minimum);

        sqlx::query!(
            "INSERT OR REPLACE INTO project_rate_limits (id, project_id, limit_type, limit_value, current_value, reset_at)
             VALUES (?, ?, ?, ?, 0, ?)",
            Uuid::new_v4().to_string(),
            project_id,
            limit_type.as_str(),
            clamped_value,
            Self::next_reset_time(limit_type)
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

pub enum RateLimitResult {
    Allowed,
    Throttled { reset_at: DateTime<Utc>, wait_seconds: u32 },
}

// System minimums
const SYSTEM_MINIMUMS: &[(LimitType, u32)] = &[
    (LimitType::HourlyRequests, 10),      // At least 10 requests/hour
    (LimitType::ConcurrentAgents, 1),     // At least 1 concurrent agent
    (LimitType::TokensPerMinute, 1000),   // At least 1000 tokens/minute
];
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/limits/rate_limiter.rs`
- `crates/demiarch-core/src/domain/limits/throttle.rs`
- `crates/demiarch-core/src/infrastructure/db/project_rate_limits.rs`
- `crates/demiarch-gui/src/components/settings/RateLimitsPanel.tsx`

**Database Schema:**
```sql
CREATE TABLE project_rate_limits (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    limit_type TEXT NOT NULL,  -- 'hourly_requests', 'concurrent_agents', 'tokens_per_minute'
    limit_value INTEGER NOT NULL,
    current_value INTEGER DEFAULT 0,
    reset_at DATETIME NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE UNIQUE INDEX idx_project_rate_limits_unique ON project_rate_limits(project_id, limit_type);
CREATE INDEX idx_project_rate_limits_reset ON project_rate_limits(reset_at);
```

### UI Components

```tsx
// RateLimitsPanel.tsx
interface RateLimitsPanelProps {
  projectId: string;
  limits: ProjectRateLimit[];
  onUpdate: (type: LimitType, value: number) => void;
}

const RateLimitsPanel: React.FC<RateLimitsPanelProps> = ({ projectId, limits, onUpdate }) => {
  return (
    <div className="glass-panel p-4">
      <h3 className="text-teal-400 font-medium mb-4">Rate Limits</h3>
      {limits.map(limit => (
        <div key={limit.limit_type} className="mb-4">
          <div className="flex justify-between items-center">
            <span>{formatLimitType(limit.limit_type)}</span>
            <span className="text-sm text-gray-400">
              {limit.current_value} / {limit.limit_value}
            </span>
          </div>
          <div className="mt-2">
            <input
              type="range"
              min={SYSTEM_MINIMUMS[limit.limit_type]}
              max={getMaxValue(limit.limit_type)}
              value={limit.limit_value}
              onChange={e => onUpdate(limit.limit_type, parseInt(e.target.value))}
            />
          </div>
          <div className="text-xs text-gray-500">
            Resets at {format(limit.reset_at, 'HH:mm')}
          </div>
        </div>
      ))}
    </div>
  );
};
```

### Testing Requirements

- Rate limit checking tests
- Counter increment tests
- Throttling tests
- Reset tests
- Configuration tests

### References

- [Source: docs/PRD.md#Cost-Management] - Rate limiting requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Scalability] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-8.6] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
