# Story 8.3: Automatic Retry on Connectivity Restore

Status: ready-for-dev

## Story

As a user,
I want queued operations to automatically retry with exponential backoff when connection returns,
so that I don't need to manually trigger each failed operation.

## Acceptance Criteria

1. **Given** System has queued operations and detects connectivity restored
   **When** Queue processing starts
   **Then** Each pending operation is processed in FIFO order

2. **Given** Operation is being processed
   **When** Operation fails
   **Then** retry_count is incremented

3. **Given** Operation fails
   **When** Backoff is calculated
   **Then** Exponential backoff is applied: delay = 2^retry_count * base_delay (max 30 seconds)

4. **Given** Retry limit is reached
   **When** retry_count reaches max_retries (3)
   **Then** Operation status is set to 'failed'

5. **Given** Operation fails permanently
   **When** Error is recorded
   **Then** last_error is recorded in queued_operations table

6. **Given** User is notified of failure
   **When** Notification is shown
   **Then** User is notified: "Operation '{operation_type}' failed after 3 retries. See logs."

7. **Given** Operation succeeds
   **When** Status is updated
   **Then** status is set to 'completed', processed_at is set to current time

8. **Given** Success occurs
   **When** User is notified
   **Then** User sees completion notification

## Tasks / Subtasks

- [ ] Task 1: Implement queue processor (AC: #1)
  - [ ] Create QueueProcessor service
  - [ ] Process operations in FIFO order
  - [ ] Handle concurrent processing limits

- [ ] Task 2: Implement exponential backoff (AC: #2, #3)
  - [ ] Calculate backoff delay
  - [ ] Apply max delay cap (30 seconds)
  - [ ] Implement async delay

- [ ] Task 3: Implement retry logic (AC: #2, #4, #5)
  - [ ] Increment retry_count on failure
  - [ ] Check max_retries limit
  - [ ] Record last_error
  - [ ] Update status to 'failed'

- [ ] Task 4: Implement success handling (AC: #7, #8)
  - [ ] Update status to 'completed'
  - [ ] Set processed_at timestamp
  - [ ] Show success notification

- [ ] Task 5: Implement failure notifications (AC: #6)
  - [ ] Show failure notification
  - [ ] Include operation type
  - [ ] Link to logs

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Exponential backoff retry (max 3 retries, 30s max delay)
- FIFO queue processing
- User notifications for outcomes

**Reliability Requirements:**
- Automatic retry on connectivity restore
- Graceful failure handling
- Clear error messaging

### Retry Logic

```rust
pub struct QueueProcessor {
    db: Database,
    connectivity: ConnectivityMonitor,
    base_delay_ms: u64,
    max_delay_ms: u64,
    max_concurrent: usize,
}

impl QueueProcessor {
    pub async fn process_queue(&self) -> Result<()> {
        let pending = self.db.query::<QueuedOperation>(
            "SELECT * FROM queued_operations
             WHERE status = 'pending'
             ORDER BY created_at ASC"
        ).await?;

        for operation in pending {
            self.process_operation(operation).await;
        }

        Ok(())
    }

    async fn process_operation(&self, mut op: QueuedOperation) {
        // Mark as processing
        op.status = OperationStatus::Processing;
        self.db.update(&op).await.ok();

        // Attempt execution
        match self.execute_operation(&op).await {
            Ok(_) => {
                op.status = OperationStatus::Completed;
                op.processed_at = Some(Utc::now());
                self.db.update(&op).await.ok();
                self.notify_success(&op);
            }
            Err(e) => {
                op.retry_count += 1;
                op.last_error = Some(e.to_string());

                if op.retry_count >= op.max_retries {
                    op.status = OperationStatus::Failed;
                    self.db.update(&op).await.ok();
                    self.notify_failure(&op);
                } else {
                    // Calculate backoff and requeue
                    let delay = self.calculate_backoff(op.retry_count);
                    op.status = OperationStatus::Pending;
                    self.db.update(&op).await.ok();

                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    fn calculate_backoff(&self, retry_count: u32) -> u64 {
        let delay = self.base_delay_ms * 2u64.pow(retry_count);
        delay.min(self.max_delay_ms)
    }
}

// Default configuration
const DEFAULT_BASE_DELAY_MS: u64 = 1000;  // 1 second
const DEFAULT_MAX_DELAY_MS: u64 = 30000;  // 30 seconds
const DEFAULT_MAX_RETRIES: u32 = 3;
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/offline/processor.rs`
- `crates/demiarch-core/src/domain/offline/retry.rs`
- `crates/demiarch-gui/src/hooks/useQueueProcessor.ts`

**Database Updates:**
```sql
-- Update operation on failure
UPDATE queued_operations
SET retry_count = retry_count + 1,
    last_error = ?,
    status = CASE WHEN retry_count >= max_retries THEN 'failed' ELSE 'pending' END
WHERE id = ?;

-- Update operation on success
UPDATE queued_operations
SET status = 'completed',
    processed_at = CURRENT_TIMESTAMP
WHERE id = ?;
```

### Testing Requirements

- FIFO processing tests
- Exponential backoff calculation tests
- Retry limit tests
- Success/failure notification tests

### References

- [Source: docs/PRD.md#Offline-Support] - Offline requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Reliability] - Retry details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-8.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
