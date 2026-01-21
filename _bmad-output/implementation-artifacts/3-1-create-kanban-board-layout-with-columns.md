# Story 3.1: Create Kanban Board Layout with Columns

Status: ready-for-dev

## Story

As a user,
I want to see a kanban board with columns representing project phases (Discovery, Planning, Building, Complete),
so that I can visualize my project's feature workflow.

## Acceptance Criteria

1. **Given** User has created a project with phases from Epic 2
   **When** User opens kanban board in GUI
   **Then** Board displays with 4 columns: "Discovery", "Planning", "Building", "Complete"

2. **Given** Kanban board is displayed
   **When** Features exist in phases
   **Then** Each column shows feature cards linked to that phase (from features table)

3. **Given** Column contains features
   **When** Column header is displayed
   **Then** Column header displays feature count (e.g., "Planning (3)")

4. **Given** Kanban board is rendered
   **When** Styling is applied
   **Then** Layout uses glassmorphism panels with custom design system colors

5. **Given** No features exist in a column
   **When** Column is displayed
   **Then** Empty columns show "No features in this phase" message

6. **Given** Different screen sizes
   **When** Board is viewed
   **Then** Board is responsive: 4 columns on desktop, 2 on tablet, 1 on mobile

## Tasks / Subtasks

- [ ] Task 1: Create KanbanBoard container component (AC: #1, #4)
  - [ ] Create KanbanBoard React component
  - [ ] Implement 4-column grid layout
  - [ ] Apply glassmorphism styling (backdrop-blur, rgba backgrounds)
  - [ ] Use design system colors (bg-surface, teal accents)

- [ ] Task 2: Create KanbanColumn component (AC: #2, #3, #5)
  - [ ] Create KanbanColumn component
  - [ ] Implement column header with name and count
  - [ ] Add feature card container
  - [ ] Handle empty state display

- [ ] Task 3: Create ColumnHeader component (AC: #3)
  - [ ] Create ColumnHeader component
  - [ ] Display phase name
  - [ ] Show feature count dynamically
  - [ ] Style with design system

- [ ] Task 4: Implement responsive layout (AC: #6)
  - [ ] Add Tailwind responsive breakpoints
  - [ ] Desktop: 4 columns (grid-cols-4)
  - [ ] Tablet: 2 columns (md:grid-cols-2)
  - [ ] Mobile: 1 column (sm:grid-cols-1)

- [ ] Task 5: Wire to Tauri commands
  - [ ] Create get_phases Tauri command
  - [ ] Create get_features_by_phase Tauri command
  - [ ] Load data on component mount

- [ ] Task 6: Create Zustand store for kanban
  - [ ] Create kanbanStore with phases and features
  - [ ] Implement loading state
  - [ ] Handle data refresh

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- React component composition
- Zustand for state management
- Tauri invoke for data fetching

**Design System Requirements:**
- Glassmorphism: backdrop-blur-20, rgba backgrounds with 0.1-0.2 opacity
- Colors: bg-surface (#253346), teal accents (#00f5d4)
- Typography: IBM Plex Sans
- Responsive breakpoints per UX specification

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/kanban/KanbanBoard.tsx`
- `crates/demiarch-gui/src/components/kanban/KanbanColumn.tsx`
- `crates/demiarch-gui/src/components/kanban/ColumnHeader.tsx`
- `crates/demiarch-gui/src/stores/kanbanStore.ts`

**Responsive Breakpoints:**
- Mobile: < 640px (1 column)
- Tablet: 640px - 1024px (2 columns)
- Desktop: > 1024px (4 columns)

### Testing Requirements

- Component tests for KanbanBoard, KanbanColumn
- Responsive layout tests
- Snapshot tests for styling
- Accessibility tests

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Kanban-Board] - Kanban UX specifications
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design-System] - Design system
- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
