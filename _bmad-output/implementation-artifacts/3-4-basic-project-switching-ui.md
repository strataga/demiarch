# Story 3.4: Basic Project Switching UI

Status: ready-for-dev

## Story

As a user with multiple projects,
I want to switch between projects via a dropdown or keyboard shortcuts,
so that I can work on different projects without restarting Demiarch.

## Acceptance Criteria

1. **Given** User has created multiple projects and is in GUI
   **When** User clicks project dropdown or presses Cmd/Ctrl + 1-5
   **Then** Dropdown shows list of all projects with names and status

2. **Given** Project dropdown is open
   **When** User selects a project
   **Then** Selecting a project loads that project's data (phases, features, chat)

3. **Given** Project is selected
   **When** UI is updated
   **Then** Active project is visually indicated with teal accent in navigation

4. **Given** Keyboard shortcuts are enabled
   **When** User presses Cmd/Ctrl + 1-5
   **Then** Shortcut Cmd/Ctrl + 1-5 switches to projects 1-5 if they exist

5. **Given** User switches projects
   **When** Previous project state is preserved
   **Then** Current project state is preserved (active chat, kanban view, focused card)

6. **Given** Project switch is initiated
   **When** Data loads
   **Then** Switch completes within 1 second

## Tasks / Subtasks

- [ ] Task 1: Create ProjectSelector dropdown component (AC: #1, #3)
  - [ ] Create ProjectSelector component
  - [ ] Display dropdown with project list
  - [ ] Show project name and status indicator
  - [ ] Highlight active project with teal accent

- [ ] Task 2: Implement project data loading (AC: #2)
  - [ ] Create switch_project Tauri command
  - [ ] Load phases, features, chat for selected project
  - [ ] Update all stores with new project data

- [ ] Task 3: Implement keyboard shortcuts (AC: #4)
  - [ ] Add useHotkeys hook or similar
  - [ ] Register Cmd/Ctrl + 1-5 shortcuts
  - [ ] Map to first 5 projects
  - [ ] Handle missing projects gracefully

- [ ] Task 4: Implement state preservation (AC: #5)
  - [ ] Store current project state before switch
  - [ ] Save: active chat ID, kanban scroll position, focused card
  - [ ] Restore state when returning to project

- [ ] Task 5: Optimize switch performance (AC: #6)
  - [ ] Implement lazy loading for project data
  - [ ] Cache recently used project data
  - [ ] Ensure switch under 1 second

- [ ] Task 6: Wire to navigation header
  - [ ] Add ProjectSelector to Header component
  - [ ] Position in navigation area
  - [ ] Handle loading states

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- State preservation across project switches
- Lazy loading for performance
- Keyboard shortcuts for power users

**Performance Requirements:**
- Switch completes within 1 second (NFR12)
- State preserved without data loss

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/navigation/ProjectSelector.tsx`
- `crates/demiarch-gui/src/stores/projectStore.ts`
- `crates/demiarch-gui/src/hooks/useKeyboardShortcuts.ts`

**State Preservation Structure:**
```typescript
interface ProjectState {
  projectId: string;
  activeChatId?: string;
  kanbanScrollPosition?: number;
  focusedCardId?: string;
  lastViewedPhase?: string;
}

// Store in localStorage or IndexedDB for persistence
```

**Keyboard Shortcuts:**
```typescript
// Using react-hotkeys-hook
useHotkeys('mod+1', () => switchToProject(0));
useHotkeys('mod+2', () => switchToProject(1));
// ... etc
```

### Testing Requirements

- Dropdown interaction tests
- Keyboard shortcut tests
- State preservation tests
- Performance tests for switch time

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Multi-Project] - Multi-project UX
- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
