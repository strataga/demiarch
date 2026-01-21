# Story 1.5: Implement Basic GUI Entry Point

Status: ready-for-dev

## Story

As a non-technical user,
I want to launch a desktop application with a graphical interface,
so that I can interact with Demiarch visually without using the terminal.

## Acceptance Criteria

1. **Given** Demiarch is installed with GUI dependencies
   **When** User launches `demiarch gui` or clicks desktop icon
   **Then** Tauri window opens with default dark theme

2. **Given** Tauri window opens
   **When** Window is displayed
   **Then** Window displays navigation header with project selector

3. **Given** No projects exist
   **When** Window content loads
   **Then** Window shows empty state with "Create your first project" prompt

4. **Given** Window is displayed
   **When** User resizes window
   **Then** Window is resizable with minimum dimensions 800x600

5. **Given** Window content loads
   **When** React app initializes
   **Then** React app loads without JavaScript errors in console

## Tasks / Subtasks

- [ ] Task 1: Configure Tauri 2.1 application (AC: #1, #4)
  - [ ] Verify tauri.conf.json configuration
  - [ ] Set window minimum dimensions (800x600)
  - [ ] Configure dark theme as default
  - [ ] Set application title and icon

- [ ] Task 2: Implement navigation header (AC: #2)
  - [ ] Create Header component with glassmorphism styling
  - [ ] Add project selector dropdown placeholder
  - [ ] Apply custom design system colors (bg-deep, teal accents)

- [ ] Task 3: Implement empty state (AC: #3)
  - [ ] Create EmptyState component
  - [ ] Design "Create your first project" prompt
  - [ ] Add call-to-action button styling

- [ ] Task 4: Setup React app structure (AC: #5)
  - [ ] Configure Zustand store structure
  - [ ] Setup React Router for navigation
  - [ ] Add error boundary for runtime errors
  - [ ] Verify no console errors on load

- [ ] Task 5: Implement design system foundation
  - [ ] Configure Tailwind CSS with custom color palette
  - [ ] Add CSS custom properties for design tokens
  - [ ] Create GlassPanel base component
  - [ ] Setup IBM Plex Sans and Fira Code fonts

- [ ] Task 6: Wire Tauri commands
  - [ ] Create initial Tauri command bindings
  - [ ] Setup invoke system for Rust ↔ TypeScript communication
  - [ ] Add basic project list command binding

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Tauri 2.1 for desktop application
- React 18.3+ with TypeScript 5.6.0
- Zustand 5.0.0 for state management
- Tailwind CSS 3.4 + shadcn/ui components

**Design System:**
- Color palette: bg-deep (#0d1b2a), teal (#00f5d4), magenta (#f72585), amber (#ffc300)
- Typography: IBM Plex Sans (UI), Fira Code (code)
- Visual language: Glassmorphism panels, neural network aesthetic

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src-tauri/` - Tauri Rust backend
- `crates/demiarch-gui/src/` - React frontend
- `crates/demiarch-gui/src/components/` - React components
- `crates/demiarch-gui/src/stores/` - Zustand stores
- `crates/demiarch-gui/src/styles/` - CSS and Tailwind config

**Naming Conventions:**
- React components: PascalCase (e.g., `Header.tsx`, `EmptyState.tsx`)
- TypeScript files: camelCase for non-components
- CSS: Tailwind utility classes + CSS custom properties

### Testing Requirements

- Component tests with React Testing Library
- Tauri command integration tests
- Visual regression tests for design system
- Accessibility tests for keyboard navigation

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design-System] - Design system details
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend] - Frontend architecture
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.5] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
