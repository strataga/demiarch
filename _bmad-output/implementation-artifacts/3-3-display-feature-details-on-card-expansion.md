# Story 3.3: Display Feature Details on Card Expansion

Status: ready-for-dev

## Story

As a user,
I want to expand a kanban card to see feature details, requirements, and linked conversation,
so that I can review what was discussed without navigating away.

## Acceptance Criteria

1. **Given** Kanban board is displaying features
   **When** User clicks on a card
   **Then** Modal/panel opens showing: feature name, description, acceptance_criteria, priority

2. **Given** Feature detail modal is open
   **When** User wants to see conversation
   **Then** "View Conversation" button links to chat history that created this feature

3. **Given** Feature detail modal is open
   **When** User wants to see code
   **Then** "View Code" button shows generated files for this feature

4. **Given** Feature detail modal is open
   **When** User wants to close
   **Then** Modal closes with Escape key or clicking outside

5. **Given** Feature detail modal is open
   **When** User views details
   **Then** Details are read-only but user can navigate to related views

## Tasks / Subtasks

- [ ] Task 1: Create FeatureDetailModal component (AC: #1, #4)
  - [ ] Create modal component with glassmorphism styling
  - [ ] Display feature name prominently
  - [ ] Show description section
  - [ ] Display acceptance criteria as list
  - [ ] Show priority indicator
  - [ ] Implement close on Escape and outside click

- [ ] Task 2: Implement modal trigger (AC: #1)
  - [ ] Add onClick handler to KanbanCard
  - [ ] Open modal with feature data
  - [ ] Animate modal entrance

- [ ] Task 3: Create View Conversation link (AC: #2)
  - [ ] Add "View Conversation" button
  - [ ] Query chat_messages for feature creation context
  - [ ] Navigate to chat view with filter

- [ ] Task 4: Create View Code link (AC: #3)
  - [ ] Add "View Code" button
  - [ ] Query generated_code for feature files
  - [ ] Display file list or navigate to code view

- [ ] Task 5: Style modal for read-only view (AC: #5)
  - [ ] Make all fields read-only
  - [ ] Add navigation buttons only
  - [ ] Clear visual hierarchy

- [ ] Task 6: Wire to Tauri commands
  - [ ] Create get_feature_details Tauri command
  - [ ] Create get_feature_chat_context Tauri command
  - [ ] Create get_feature_generated_files Tauri command

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Modal component with overlay
- Read-only detail view
- Navigation links to related views

**Design System Requirements:**
- Glassmorphism modal styling
- Backdrop blur with overlay
- Teal accent for buttons
- Amber for priority indicators

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/kanban/FeatureDetailModal.tsx`
- `crates/demiarch-gui/src/components/shared/Modal.tsx`
- `crates/demiarch-gui/src/hooks/useModal.ts`

**Modal Structure:**
```tsx
<Modal isOpen={isOpen} onClose={onClose}>
  <ModalHeader>
    <h2>{feature.name}</h2>
    <PriorityBadge priority={feature.priority} />
  </ModalHeader>
  <ModalBody>
    <Section title="Description">{feature.description}</Section>
    <Section title="Acceptance Criteria">
      <AcceptanceCriteriaList items={feature.acceptance_criteria} />
    </Section>
  </ModalBody>
  <ModalFooter>
    <Button onClick={viewConversation}>View Conversation</Button>
    <Button onClick={viewCode}>View Code</Button>
  </ModalFooter>
</Modal>
```

### Testing Requirements

- Modal open/close tests
- Keyboard accessibility tests (Escape to close)
- Navigation link tests
- Content display tests

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Progressive-Disclosure] - Detail expansion pattern
- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
