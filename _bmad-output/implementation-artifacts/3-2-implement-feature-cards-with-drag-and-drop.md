# Story 3.2: Implement Feature Cards with Drag-and-Drop

Status: ready-for-dev

## Story

As a user,
I want to drag feature cards between kanban columns to update their status,
so that I can track progress visually with satisfying interactions.

## Acceptance Criteria

1. **Given** Kanban board is displaying features
   **When** User drags a card and hovers over a column
   **Then** Column highlights with teal glow indicating drop target

2. **Given** User is dragging a card
   **When** User releases card over valid column
   **Then** Card snaps into column with smooth animation (60fps, no lag)

3. **Given** Card is dropped in new column
   **When** Database is updated
   **Then** Feature's phase_id in database is updated to new column's phase

4. **Given** Card is moved
   **When** Card is displayed in new column
   **Then** Card shows updated status badge reflecting new phase

5. **Given** Card move is processed
   **When** User completes drag
   **Then** Update happens within 2 seconds of drop

6. **Given** User is dragging a card
   **When** User wants to cancel
   **Then** User can cancel drag with Escape key or by clicking outside board

## Tasks / Subtasks

- [ ] Task 1: Create KanbanCard component (AC: #4)
  - [ ] Create KanbanCard component
  - [ ] Display feature name, priority, status badge
  - [ ] Apply glassmorphism card styling
  - [ ] Add hover and active states

- [ ] Task 2: Implement drag-and-drop library (AC: #1, #2)
  - [ ] Add @dnd-kit/core and @dnd-kit/sortable
  - [ ] Configure DndContext provider
  - [ ] Implement useDraggable for cards
  - [ ] Implement useDroppable for columns

- [ ] Task 3: Implement drop target highlighting (AC: #1)
  - [ ] Add isOver state detection
  - [ ] Apply teal glow on hover (box-shadow with teal)
  - [ ] Smooth transition for highlight

- [ ] Task 4: Implement smooth animations (AC: #2)
  - [ ] Configure spring animation for snap
  - [ ] Ensure 60fps performance
  - [ ] Add CSS transitions for card movement

- [ ] Task 5: Implement database update (AC: #3, #5)
  - [ ] Create update_feature_phase Tauri command
  - [ ] Call on drop completion
  - [ ] Handle optimistic updates
  - [ ] Roll back on failure

- [ ] Task 6: Implement drag cancellation (AC: #6)
  - [ ] Handle Escape key press
  - [ ] Handle click outside board
  - [ ] Reset card position on cancel

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- @dnd-kit for drag-and-drop (React-friendly, performant)
- Optimistic updates with rollback
- CSS transforms for GPU-accelerated animations

**Performance Requirements:**
- Drag-and-drop at 60fps with no lag (NFR12)
- Update within 2 seconds of drop

### Drag-and-Drop Library

Using @dnd-kit because:
- Accessible by default
- Performance optimized
- React-first design
- Flexible collision detection

```tsx
// Example structure
<DndContext onDragEnd={handleDragEnd}>
  {columns.map(column => (
    <Droppable id={column.id}>
      {features.map(feature => (
        <Draggable id={feature.id}>
          <KanbanCard feature={feature} />
        </Draggable>
      ))}
    </Droppable>
  ))}
</DndContext>
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-gui/src/components/kanban/KanbanCard.tsx`
- `crates/demiarch-gui/src/components/kanban/DraggableCard.tsx`
- `crates/demiarch-gui/src/components/kanban/DroppableColumn.tsx`
- `crates/demiarch-gui/src/hooks/useDragAndDrop.ts`

**Animation CSS:**
```css
.card-dragging {
  transform: scale(1.02);
  box-shadow: 0 10px 20px rgba(0, 245, 212, 0.3);
}

.column-drop-target {
  box-shadow: 0 0 20px rgba(0, 245, 212, 0.5);
}
```

### Testing Requirements

- Drag-and-drop interaction tests
- Animation performance tests
- Database update integration tests
- Keyboard cancellation tests

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Effortless-Interactions] - Drag interaction requirements
- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
