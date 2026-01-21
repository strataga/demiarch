# Story 8.4: Graceful Degradation for Offline Features

Status: ready-for-dev

## Story

As a user,
I want non-essential features to be disabled when offline,
so that I can continue core development without confusion.

## Acceptance Criteria

1. **Given** User is offline
   **When** System enters degraded mode
   **Then** LLM-dependent features show "Unavailable offline" message with amber warning

2. **Given** Degraded mode is active
   **When** Offline features are checked
   **Then** Offline-capable features remain fully functional (file editing, project viewing, checkpoint management)

3. **Given** User attempts offline feature
   **When** Feature is triggered
   **Then** User sees clear explanation: "This feature requires internet connectivity. Your operation has been queued."

4. **Given** Operation is queued
   **When** Confirmation is shown
   **Then** Queue confirmation is shown: "Operation queued. Will run when online."

5. **Given** Connectivity restores
   **When** Features are re-enabled
   **Then** All features are re-enabled without requiring restart

## Tasks / Subtasks

- [ ] Task 1: Define feature availability (AC: #1, #2)
  - [ ] Create feature availability matrix
  - [ ] Tag features as online-required or offline-capable
  - [ ] Implement availability checks

- [ ] Task 2: Implement degraded UI (AC: #1)
  - [ ] Disable online-required buttons
  - [ ] Show amber warning indicators
  - [ ] Display "Unavailable offline" tooltip

- [ ] Task 3: Implement offline explanations (AC: #3, #4)
  - [ ] Show explanation modal on attempt
  - [ ] Auto-queue operation
  - [ ] Show queue confirmation toast

- [ ] Task 4: Implement seamless restoration (AC: #5)
  - [ ] Listen for connectivity restore
  - [ ] Re-enable all features
  - [ ] No restart required

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Graceful degradation without errors
- Clear offline messaging
- Seamless restoration

**Usability Requirements:**
- Offline-capable features work fully
- Clear indication of unavailable features
- Automatic queuing for attempted operations

### Feature Availability Matrix

```rust
pub enum FeatureAvailability {
    AlwaysAvailable,    // Works offline
    OnlineRequired,     // Needs network
    DegradedOffline,    // Partial functionality offline
}

pub struct FeatureMatrix {
    features: HashMap<Feature, FeatureAvailability>,
}

impl FeatureMatrix {
    pub fn new() -> Self {
        let mut features = HashMap::new();

        // Always available (offline-capable)
        features.insert(Feature::FileEditing, FeatureAvailability::AlwaysAvailable);
        features.insert(Feature::ProjectViewing, FeatureAvailability::AlwaysAvailable);
        features.insert(Feature::CheckpointManagement, FeatureAvailability::AlwaysAvailable);
        features.insert(Feature::KanbanBoard, FeatureAvailability::AlwaysAvailable);
        features.insert(Feature::LocalSearch, FeatureAvailability::AlwaysAvailable);

        // Online required
        features.insert(Feature::CodeGeneration, FeatureAvailability::OnlineRequired);
        features.insert(Feature::ChatWithAI, FeatureAvailability::OnlineRequired);
        features.insert(Feature::SkillSearch, FeatureAvailability::OnlineRequired);
        features.insert(Feature::PluginInstall, FeatureAvailability::OnlineRequired);
        features.insert(Feature::Sync, FeatureAvailability::OnlineRequired);

        // Degraded offline (read-only, cached data)
        features.insert(Feature::SkillViewing, FeatureAvailability::DegradedOffline);
        features.insert(Feature::CostDashboard, FeatureAvailability::DegradedOffline);

        Self { features }
    }

    pub fn is_available(&self, feature: Feature, is_offline: bool) -> bool {
        match self.features.get(&feature) {
            Some(FeatureAvailability::AlwaysAvailable) => true,
            Some(FeatureAvailability::OnlineRequired) => !is_offline,
            Some(FeatureAvailability::DegradedOffline) => true,  // Read-only
            None => true,  // Unknown features default to available
        }
    }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/offline/feature_matrix.rs`
- `crates/demiarch-core/src/domain/offline/degradation.rs`
- `crates/demiarch-gui/src/hooks/useFeatureAvailability.ts`
- `crates/demiarch-gui/src/components/common/OfflineFeatureGuard.tsx`

### UI Components

```tsx
// OfflineFeatureGuard.tsx
interface OfflineFeatureGuardProps {
  feature: Feature;
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

const OfflineFeatureGuard: React.FC<OfflineFeatureGuardProps> = ({
  feature,
  children,
  fallback
}) => {
  const { isOffline } = useConnectivity();
  const { isAvailable } = useFeatureAvailability(feature, isOffline);

  if (!isAvailable) {
    return fallback || (
      <div className="flex items-center gap-2 text-amber-400">
        <WifiOff className="w-4 h-4" />
        <span>Unavailable offline</span>
      </div>
    );
  }

  return <>{children}</>;
};

// Usage
<OfflineFeatureGuard feature={Feature.CodeGeneration}>
  <GenerateCodeButton />
</OfflineFeatureGuard>
```

### Testing Requirements

- Feature availability tests
- Degraded UI tests
- Queue-on-attempt tests
- Seamless restoration tests

### References

- [Source: docs/PRD.md#Offline-Support] - Offline requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Reliability] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-8.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
