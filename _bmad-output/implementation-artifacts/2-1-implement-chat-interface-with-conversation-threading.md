# Story 2.1: Implement Chat Interface with Conversation Threading

Status: ready-for-dev

## Story

As a user,
I want to chat with Demiarch's AI through a conversational interface with message history,
so that I can discuss my project requirements naturally and see our conversation context.

## Acceptance Criteria

1. **Given** User has created a project and opened the chat interface (CLI or GUI)
   **When** User types a message and sends it
   **Then** Message appears immediately in chat stream with user avatar and timestamp

2. **Given** Message is sent
   **When** AI processes the message
   **Then** AI response appears within 5 seconds with typing indicator shown during processing

3. **Given** Chat messages are exchanged
   **When** Messages are stored
   **Then** All messages are saved to chat_messages table with project_id, role, content, token_count, model

4. **Given** Chat interface is displayed
   **When** New messages arrive
   **Then** Chat scrolls automatically to latest message

5. **Given** Long conversation history exists
   **When** User views chat
   **Then** User can scroll up to see full conversation history

## Tasks / Subtasks

- [ ] Task 1: Create chat_messages table schema (AC: #3)
  - [ ] Add migration for chat_messages table
  - [ ] Include columns: id, project_id, role, content, token_count, model, created_at
  - [ ] Add foreign key to projects table
  - [ ] Create indexes for project_id queries

- [ ] Task 2: Implement ChatMessage domain model (AC: #3)
  - [ ] Create ChatMessage struct with all fields
  - [ ] Implement ChatMessageRepository trait
  - [ ] Create SQLite repository implementation

- [ ] Task 3: Implement GUI ChatContainer component (AC: #1, #4, #5)
  - [ ] Create ChatContainer React component
  - [ ] Implement message list with auto-scroll
  - [ ] Add scroll-to-bottom behavior on new messages
  - [ ] Support scrolling through history

- [ ] Task 4: Implement MessageBubble component (AC: #1)
  - [ ] Create MessageBubble component with user/AI styling
  - [ ] Add timestamp display
  - [ ] Apply design system colors (user: teal accent, AI: magenta accent)
  - [ ] Add avatar/icon indicators

- [ ] Task 5: Implement ChatInput component (AC: #1)
  - [ ] Create ChatInput with text area
  - [ ] Add send button with loading state
  - [ ] Handle Enter key to send
  - [ ] Disable input during AI processing

- [ ] Task 6: Implement typing indicator (AC: #2)
  - [ ] Create TypingIndicator component
  - [ ] Show during AI processing
  - [ ] Animate with pulsing dots

- [ ] Task 7: Wire chat to Tauri commands
  - [ ] Create send_message Tauri command
  - [ ] Create get_messages Tauri command
  - [ ] Implement real-time message updates

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain-driven design for ChatMessage
- Repository pattern for data access
- Tauri invoke system for Rust ↔ TypeScript communication

**UI Requirements:**
- Glassmorphism styling for chat container
- Design system colors: user (teal), AI (magenta)
- IBM Plex Sans for message text
- Smooth animations for message appearance

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/chat/` - Chat domain module
- `crates/demiarch-core/src/infrastructure/db/chat_messages.rs` - Repository
- `crates/demiarch-gui/src/components/chat/` - Chat components
- `crates/demiarch-gui/src/stores/chatStore.ts` - Zustand chat store

**Database Schema:**
```sql
CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    token_count INTEGER,
    model TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

### Testing Requirements

- Component tests for ChatContainer, MessageBubble, ChatInput
- Integration tests for message persistence
- Visual tests for styling
- Accessibility tests for keyboard navigation

### References

- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Chat-Interface] - UX specifications
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.1] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
