# Story 5.6: Full Agent Visualization with Hierarchy Tree

Status: ready-for-dev

## Story

As a technical user,
I want to see complete Russian Doll agent execution tree with detailed metrics,
so that I can understand what each agent did and optimize performance.

## Acceptance Criteria

1. **Given** User has triggered code generation and wants to inspect agent execution
   **When** User opens "Agent Execution" panel or clicks on agent status
   **Then** Neural network visualization displays with concentric rings:
     - Outer ring: Orchestrator (teal)
     - Middle ring: Planner (magenta)
     - Inner ring: Coder, Reviewer, Tester (amber)

2. **Given** Neural network is displayed
   **When** Agent nodes are shown
   **Then** Each agent node shows: status, execution time, token usage, cost

3. **Given** Agents are connected
   **When** Connections are displayed
   **Then** Connections between nodes animate with flowing particles

4. **Given** User interacts with visualization
   **When** User clicks on an agent node
   **Then** Detail panel shows: agent_type, input_context, output_result, error_message (if failed)

5. **Given** Detail panel is open
   **When** User views execution tree
   **Then** User can see full execution tree with parent-child relationships

6. **Given** Technical details are displayed
   **When** User views metrics
   **Then** Technical details are shown in readable format (e.g., "Coder generated 238 tokens in 3.2s")

## Tasks / Subtasks

- [ ] Task 1: Create NeuralNetwork SVG component (AC: #1)
  - [ ] Create SVG-based concentric ring layout
  - [ ] Position Orchestrator at outer ring
  - [ ] Position Planners at middle ring
  - [ ] Position Workers at inner ring
  - [ ] Apply design system colors

- [ ] Task 2: Create AgentNode component (AC: #1, #2)
  - [ ] Create circular node component
  - [ ] Display status indicator
  - [ ] Show mini-metrics (time, tokens, cost)
  - [ ] Apply pulsing animation for active agents

- [ ] Task 3: Create ConnectionPath component (AC: #3)
  - [ ] Create SVG path between nodes
  - [ ] Apply gradient stroke
  - [ ] Implement flowing particle animation
  - [ ] Animate on data flow

- [ ] Task 4: Create AgentDetail panel (AC: #4, #5, #6)
  - [ ] Create detail panel component
  - [ ] Display agent_type and status
  - [ ] Show input_context (collapsible)
  - [ ] Show output_result (collapsible)
  - [ ] Show error_message if failed
  - [ ] Format metrics readably

- [ ] Task 5: Implement node click handling (AC: #4)
  - [ ] Add onClick to AgentNode
  - [ ] Open AgentDetail panel
  - [ ] Pass agent execution data

- [ ] Task 6: Wire to real-time updates
  - [ ] Subscribe to agent execution events
  - [ ] Update node status in real-time
  - [ ] Animate connections on delegation

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- SVG-based neural network visualization
- Real-time updates via event subscription
- Design system colors and animations

**Design System Requirements:**
- Colors: teal (#00f5d4) for Orchestrator, magenta (#f72585) for Planner, amber (#ffc300) for Workers
- Animations: node-pulse, connection-flow, particle-move
- Glassmorphism for detail panel

### Neural Network Layout

```
          ┌──────────────────────┐
          │   Orchestrator (teal) │  ← Outer ring
          └──────────┬───────────┘
                     │
          ┌──────────┴───────────┐
          │   Planner (magenta)   │  ← Middle ring
          └──────────┬───────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
┌───┴───┐       ┌────┴────┐      ┌────┴────┐
│ Coder │       │Reviewer │      │ Tester  │  ← Inner ring
│(amber)│       │ (amber) │      │ (amber) │
└───────┘       └─────────┘      └─────────┘
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/agents/NeuralNetwork.tsx`
- `crates/demiarch-gui/src/components/agents/AgentNode.tsx`
- `crates/demiarch-gui/src/components/agents/ConnectionPath.tsx`
- `crates/demiarch-gui/src/components/agents/AgentDetail.tsx`
- `crates/demiarch-gui/src/components/agents/ParticleAnimation.tsx`

**Animation CSS:**
```css
@keyframes connection-flow {
  0% { stroke-dashoffset: 20; }
  100% { stroke-dashoffset: 0; }
}

@keyframes particle-move {
  0% { offset-distance: 0%; }
  100% { offset-distance: 100%; }
}

.agent-node-active {
  animation: node-pulse 2s ease-in-out infinite;
}
```

### Testing Requirements

- SVG rendering tests
- Animation performance tests (60fps)
- Click interaction tests
- Real-time update tests

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Agent-Visualization] - Visualization requirements
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design-System] - Design system
- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.6] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
