# Story 8.1: Detect Network Connectivity and Enter Offline Mode

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically detect when internet is unavailable,
so that the system degrades gracefully without errors.

## Acceptance Criteria

1. **Given** Demiarch is running and making LLM requests
   **When** Network connectivity is lost (timeout, DNS failure, no route)
   **Then** System sets offline_mode=true and displays degraded UI banner

2. **Given** Offline mode is activated
   **When** User is notified
   **Then** User sees message: "Offline mode activated. Operations will queue until connection restores."

3. **Given** LLM requests are pending
   **When** Offline mode is active
   **Then** All pending LLM requests are queued to queued_operations table

4. **Given** System is offline
   **When** Connectivity is detected again
   **Then** System exits offline mode and processes queued operations

5. **Given** Connection restores
   **When** User is notified
   **Then** User sees: "Connection restored. Processing 3 queued operations."

## Tasks / Subtasks

- [ ] Task 1: Implement connectivity detection (AC: #1)
  - [ ] Monitor LLM request responses
  - [ ] Detect timeout errors
  - [ ] Detect DNS failures
  - [ ] Detect network unreachable errors

- [ ] Task 2: Implement offline mode state (AC: #1, #2)
  - [ ] Create offline_mode flag in application state
  - [ ] Trigger UI banner on mode change
  - [ ] Display notification message

- [ ] Task 3: Implement request queuing (AC: #3)
  - [ ] Create queued_operations table
  - [ ] Queue failed requests automatically
  - [ ] Preserve request context

- [ ] Task 4: Implement connectivity restoration (AC: #4, #5)
  - [ ] Periodically check connectivity
  - [ ] Exit offline mode when restored
  - [ ] Process queued operations
  - [ ] Show restoration notification

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Automatic offline detection
- Graceful degradation
- Operation queuing for retry

**Reliability Requirements:**
- Exponential backoff for retry
- Model fallback support
- No data loss during offline

### Connectivity Detection

```rust
pub struct ConnectivityMonitor {
    is_offline: AtomicBool,
    last_check: AtomicU64,
    check_interval_ms: u64,
}

impl ConnectivityMonitor {
    pub fn handle_error(&self, error: &reqwest::Error) -> bool {
        if error.is_timeout() || error.is_connect() || is_dns_error(error) {
            self.enter_offline_mode();
            true
        } else {
            false
        }
    }

    pub fn enter_offline_mode(&self) {
        if !self.is_offline.swap(true, Ordering::SeqCst) {
            // First time entering offline mode
            info!("Entering offline mode");
            self.notify_ui(OfflineEvent::Entered);
        }
    }

    pub fn exit_offline_mode(&self) {
        if self.is_offline.swap(false, Ordering::SeqCst) {
            // Exiting offline mode
            info!("Exiting offline mode");
            self.notify_ui(OfflineEvent::Restored);
        }
    }

    pub async fn check_connectivity(&self) -> bool {
        // Simple connectivity check
        match reqwest::get("https://api.openrouter.ai/health").await {
            Ok(resp) if resp.status().is_success() => {
                self.exit_offline_mode();
                true
            }
            _ => false,
        }
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/network/connectivity.rs`
- `crates/demiarch-core/src/domain/network/offline_mode.rs`
- `crates/demiarch-core/src/infrastructure/db/queued_operations.rs`
- `crates/demiarch-gui/src/components/common/OfflineBanner.tsx`

**Database Schema:**
```sql
CREATE TABLE queued_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,  -- 'llm_request', 'sync', 'plugin_install'
    payload TEXT NOT NULL,  -- JSON
    status TEXT DEFAULT 'pending',  -- 'pending', 'processing', 'completed', 'failed'
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    processed_at DATETIME
);

CREATE INDEX idx_queued_operations_status ON queued_operations(status);
```

### Testing Requirements

- Network error detection tests
- Offline mode transition tests
- Request queuing tests
- Connectivity restoration tests

### References

- [Source: docs/PRD.md#Offline-Support] - Offline requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Reliability] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-8.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
