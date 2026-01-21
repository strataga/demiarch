# Story 3.5: Basic Agent Status Visualization

Status: ready-for-dev

## Story

As a user,
I want to see basic agent activity status when code generation is running,
so that I know what's happening without technical details.

## Acceptance Criteria

1. **Given** User has triggered code generation for a feature
   **When** Agents are working on a feature
   **Then** Panel shows simplified status: "AI agents working on this feature"

2. **Given** Agents are active
   **When** Status is displayed
   **Then** Status pulses with amber color to indicate activity

3. **Given** Generation completes successfully
   **When** Status is updated
   **Then** Status changes to "Feature complete" with teal color

4. **Given** Generation completes
   **When** Summary is displayed
   **Then** Panel shows summary: "Generated 5 files, 3 components, 2 API routes"

5. **Given** Generation fails
   **When** Error occurs
   **Then** Status shows "Generation failed - see logs" with error message

6. **Given** Generation has failed
   **When** User wants to retry
   **Then** Panel offers "Retry" button

## Tasks / Subtasks

- [ ] Task 1: Create AgentStatusPanel component (AC: #1, #2)
  - [ ] Create AgentStatusPanel component
  - [ ] Display simplified status message
  - [ ] Apply amber pulsing animation for activity
  - [ ] Position in appropriate location (sidebar or modal)

- [ ] Task 2: Implement status states (AC: #1, #3, #5)
  - [ ] Define AgentStatus enum: idle, working, complete, failed
  - [ ] Create status message mappings
  - [ ] Implement color coding (amber: working, teal: complete, red: failed)

- [ ] Task 3: Implement pulsing animation (AC: #2)
  - [ ] Create CSS pulse animation
  - [ ] Apply amber glow effect
  - [ ] Ensure smooth 60fps animation

- [ ] Task 4: Implement completion summary (AC: #3, #4)
  - [ ] Create GenerationSummary component
  - [ ] Count and display: files, components, routes
  - [ ] Show teal success styling

- [ ] Task 5: Implement error handling (AC: #5, #6)
  - [ ] Display error message
  - [ ] Show "Retry" button
  - [ ] Implement retry functionality
  - [ ] Log errors for debugging

- [ ] Task 6: Wire to generation events
  - [ ] Subscribe to generation status events
  - [ ] Update panel on status changes
  - [ ] Handle real-time updates

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Event-driven status updates
- Simplified status for non-technical users
- Progressive disclosure (basic status → detailed logs)

**Design System Requirements:**
- Colors: amber (#ffc300) for working, teal (#00f5d4) for complete
- Pulsing animation from design system (node-pulse)
- Glassmorphism panel styling

### Status States

```typescript
type AgentStatus = 'idle' | 'working' | 'complete' | 'failed';

interface StatusConfig {
  idle: { message: '', color: 'none' },
  working: { message: 'AI agents working on this feature', color: 'amber', animate: true },
  complete: { message: 'Feature complete', color: 'teal', animate: false },
  failed: { message: 'Generation failed - see logs', color: 'red', animate: false }
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/agents/AgentStatusPanel.tsx`
- `crates/demiarch-gui/src/components/agents/GenerationSummary.tsx`
- `crates/demiarch-gui/src/stores/agentStore.ts`

**Animation CSS:**
```css
@keyframes agent-pulse {
  0%, 100% { box-shadow: 0 0 10px rgba(255, 195, 0, 0.3); }
  50% { box-shadow: 0 0 20px rgba(255, 195, 0, 0.6); }
}

.status-working {
  animation: agent-pulse 2s ease-in-out infinite;
}
```

### Testing Requirements

- Status state transition tests
- Animation rendering tests
- Error handling tests
- Retry functionality tests

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Agent-Visualization] - Agent status UX
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Visible-Progress] - Progress visibility
- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
