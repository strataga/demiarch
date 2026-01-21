# Story 8.5: Cost Alerts with Daily Limits

Status: ready-for-dev

## Story

As a user,
I want to be alerted when approaching or exceeding daily cost limits,
so that I can control spending and avoid unexpected bills.

## Acceptance Criteria

1. **Given** User has set cost_daily_limit_usd in user_preferences (default $10.00)
   **When** System tracks LLM usage throughout the day
   **Then** Daily cost is calculated from llm_usage table for current date

2. **Given** Daily cost is tracked
   **When** Cost reaches 80% of limit (cost_alert_threshold)
   **Then** cost_alerts record is created with alert_type='daily_threshold'

3. **Given** Threshold alert is created
   **When** User is notified
   **Then** User sees notification: "Daily cost alert: $8.00 of $10.00 spent. 20% remaining."

4. **Given** Cost continues to increase
   **When** Cost reaches or exceeds daily_limit
   **Then** cost_alerts record is created with alert_type='daily_limit'

5. **Given** Limit is reached
   **When** User is blocked
   **Then** User sees blocking message: "Daily cost limit reached. Generate blocked until tomorrow or increase limit."

6. **Given** Alert is shown
   **When** Alert is acknowledged
   **Then** cost_alerts.acknowledged is set to 0 initially

7. **Given** User acknowledges alert
   **When** Acknowledgment occurs
   **Then** acknowledged is set to 1 and notification dismisses

8. **Given** User wants to view history
   **When** Cost history is accessed
   **Then** User can view cost history with daily breakdowns

## Tasks / Subtasks

- [ ] Task 1: Create cost_alerts table schema (AC: #2, #4, #6, #7)
  - [ ] Add migration for cost_alerts table
  - [ ] Include columns: id, alert_type, threshold_value, current_value, limit_value, acknowledged, created_at
  - [ ] Add indexes for date and acknowledged queries

- [ ] Task 2: Implement cost tracking (AC: #1)
  - [ ] Calculate daily cost from llm_usage
  - [ ] Update running total on each request
  - [ ] Reset at midnight

- [ ] Task 3: Implement threshold alerts (AC: #2, #3)
  - [ ] Check cost against threshold (80%)
  - [ ] Create alert record
  - [ ] Show notification

- [ ] Task 4: Implement limit enforcement (AC: #4, #5)
  - [ ] Block generation when limit reached
  - [ ] Show blocking message
  - [ ] Provide option to increase limit

- [ ] Task 5: Implement acknowledgment (AC: #6, #7)
  - [ ] Add acknowledge button
  - [ ] Update acknowledged flag
  - [ ] Dismiss notification

- [ ] Task 6: Implement cost history UI (AC: #8)
  - [ ] Create CostHistoryPanel component
  - [ ] Show daily breakdowns
  - [ ] Display trends and totals

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Real-time cost tracking
- Threshold and limit alerts
- User acknowledgment workflow

**Cost Management Requirements:**
- Default daily limit: $10.00
- Alert threshold: 80% of limit
- Block on limit reached

### Cost Alert System

```rust
pub struct CostTracker {
    db: Database,
    daily_limit_usd: f64,
    alert_threshold: f64,  // 0.8 = 80%
}

impl CostTracker {
    pub async fn record_usage(&self, cost_usd: f64) -> Result<Option<CostAlert>> {
        let today = Utc::now().date_naive();

        // Get current daily cost
        let daily_cost: f64 = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(cost_usd), 0) FROM llm_usage
             WHERE DATE(created_at) = ?",
            today
        )
        .fetch_one(&self.db)
        .await?;

        let new_total = daily_cost + cost_usd;

        // Check thresholds
        if new_total >= self.daily_limit_usd {
            return Ok(Some(self.create_alert(AlertType::DailyLimit, new_total).await?));
        }

        if new_total >= self.daily_limit_usd * self.alert_threshold
            && daily_cost < self.daily_limit_usd * self.alert_threshold
        {
            return Ok(Some(self.create_alert(AlertType::DailyThreshold, new_total).await?));
        }

        Ok(None)
    }

    pub async fn can_generate(&self) -> Result<bool> {
        let today = Utc::now().date_naive();
        let daily_cost: f64 = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(cost_usd), 0) FROM llm_usage
             WHERE DATE(created_at) = ?",
            today
        )
        .fetch_one(&self.db)
        .await?;

        Ok(daily_cost < self.daily_limit_usd)
    }
}

pub enum AlertType {
    DailyThreshold,  // 80% of limit reached
    DailyLimit,      // 100% of limit reached
}

pub struct CostAlert {
    pub id: String,
    pub alert_type: AlertType,
    pub threshold_value: f64,
    pub current_value: f64,
    pub limit_value: f64,
    pub acknowledged: bool,
    pub created_at: DateTime<Utc>,
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/costs/tracker.rs`
- `crates/demiarch-core/src/domain/costs/alerts.rs`
- `crates/demiarch-core/src/infrastructure/db/cost_alerts.rs`
- `crates/demiarch-gui/src/components/costs/CostAlertBanner.tsx`
- `crates/demiarch-gui/src/components/costs/CostHistoryPanel.tsx`

**Database Schema:**
```sql
CREATE TABLE cost_alerts (
    id TEXT PRIMARY KEY,
    alert_type TEXT NOT NULL,  -- 'daily_threshold', 'daily_limit'
    threshold_value REAL NOT NULL,
    current_value REAL NOT NULL,
    limit_value REAL NOT NULL,
    acknowledged INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_cost_alerts_date ON cost_alerts(DATE(created_at));
CREATE INDEX idx_cost_alerts_acknowledged ON cost_alerts(acknowledged);

-- User preferences for cost limits
-- Already exists, add columns if needed
ALTER TABLE user_preferences ADD COLUMN cost_daily_limit_usd REAL DEFAULT 10.0;
ALTER TABLE user_preferences ADD COLUMN cost_alert_threshold REAL DEFAULT 0.8;
```

### Testing Requirements

- Cost calculation tests
- Threshold alert tests
- Limit enforcement tests
- Acknowledgment tests
- History display tests

### References

- [Source: docs/PRD.md#Cost-Management] - Cost requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Cost-Management] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-8.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
