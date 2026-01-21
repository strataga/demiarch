# Story 8.2: Queue Operations During Offline Mode

Status: ready-for-dev

## Story

As a user,
I want my operations to be automatically queued when I'm offline,
so that I can continue working and have operations executed when connectivity returns.

## Acceptance Criteria

1. **Given** User is offline and performs an action requiring network (LLM call, sync, plugin install)
   **When** Operation is initiated
   **Then** Record is created in queued_operations table with operation_type, payload JSON, status='pending'

2. **Given** Operation is queued
   **When** Retry settings are initialized
   **Then** retry_count is initialized to 0, max_retries set to 3

3. **Given** User wants to see queue
   **When** User views operations queue
   **Then** Queue displays count of pending operations

4. **Given** Queue is displayed
   **When** User views details
   **Then** Each operation shows type and status (pending/processing/completed/failed)

5. **Given** User wants to cancel
   **When** User cancels operation
   **Then** User can cancel individual operations with confirmation

## Tasks / Subtasks

- [ ] Task 1: Implement operation queuing (AC: #1, #2)
  - [ ] Create queue_operation() method
  - [ ] Serialize operation payload to JSON
  - [ ] Initialize retry settings
  - [ ] Handle different operation types

- [ ] Task 2: Implement queue display UI (AC: #3, #4)
  - [ ] Create QueuePanel component
  - [ ] Show pending operation count
  - [ ] Display operation type and status
  - [ ] Add status badges

- [ ] Task 3: Implement operation cancellation (AC: #5)
  - [ ] Add cancel button per operation
  - [ ] Show confirmation dialog
  - [ ] Update status to 'cancelled'
  - [ ] Remove from queue

- [ ] Task 4: Implement queue persistence
  - [ ] Store queue in SQLite
  - [ ] Recover queue on restart
  - [ ] Clean up completed operations

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Operation queuing for offline support
- Queue persistence across restarts
- User-controlled cancellation

**Queue Requirements:**
- All network operations queueable
- Preserve operation context
- FIFO processing order

### Operation Queue Design

```rust
pub enum OperationType {
    LlmRequest,
    Sync,
    PluginInstall,
    SkillSearch,
}

pub struct QueuedOperation {
    pub id: String,
    pub operation_type: OperationType,
    pub payload: serde_json::Value,
    pub status: OperationStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

pub enum OperationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl OperationQueue {
    pub async fn enqueue(&self, op_type: OperationType, payload: impl Serialize) -> Result<String> {
        let operation = QueuedOperation {
            id: Uuid::new_v4().to_string(),
            operation_type: op_type,
            payload: serde_json::to_value(payload)?,
            status: OperationStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            created_at: Utc::now(),
            processed_at: None,
        };

        self.db.insert(&operation).await?;
        self.notify_queue_changed();

        Ok(operation.id)
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE queued_operations SET status = 'cancelled' WHERE id = ? AND status = 'pending'",
            id
        )
        .execute(&self.db)
        .await?;

        self.notify_queue_changed();
        Ok(())
    }

    pub async fn pending_count(&self) -> Result<u32> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM queued_operations WHERE status = 'pending'"
        )
        .fetch_one(&self.db)
        .await?;

        Ok(count as u32)
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/offline/queue.rs`
- `crates/demiarch-core/src/domain/offline/operation.rs`
- `crates/demiarch-gui/src/components/offline/QueuePanel.tsx`
- `crates/demiarch-gui/src/components/offline/OperationItem.tsx`

**Database Schema:**
```sql
-- Already defined in 8-1, used here
CREATE TABLE queued_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    processed_at DATETIME
);
```

### UI Components

```tsx
// QueuePanel.tsx
interface QueuePanelProps {
  operations: QueuedOperation[];
  onCancel: (id: string) => void;
}

const QueuePanel: React.FC<QueuePanelProps> = ({ operations, onCancel }) => {
  const pending = operations.filter(op => op.status === 'pending');

  return (
    <div className="glass-panel p-4">
      <h3 className="text-teal-400 font-medium mb-2">
        Queued Operations ({pending.length})
      </h3>
      <div className="space-y-2">
        {operations.map(op => (
          <OperationItem key={op.id} operation={op} onCancel={onCancel} />
        ))}
      </div>
    </div>
  );
};
```

### Testing Requirements

- Operation queuing tests
- Queue display tests
- Cancellation tests
- Persistence tests

### References

- [Source: docs/PRD.md#Offline-Support] - Offline requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Reliability] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-8.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
