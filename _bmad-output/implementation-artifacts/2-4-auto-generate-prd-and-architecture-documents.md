# Story 2.4: Auto-Generate PRD and Architecture Documents

Status: ready-for-dev

## Story

As a user,
I want to automatically generate PRD and Architecture documents based on our conversation,
so that I can have formal project documentation without writing it manually.

## Acceptance Criteria

1. **Given** User has discussed project requirements in chat and requests document generation
   **When** AI generates PRD document
   **Then** PRD.md is created in project root with sections: Overview, Functional Requirements, Non-Functional Requirements, Architecture, Database Schema

2. **Given** PRD generation
   **When** Document is created
   **Then** Content reflects all requirements discussed in conversation

3. **Given** User requests architecture document
   **When** AI generates Architecture document
   **Then** architecture.md is created in project root with sections: System Overview, Tech Stack, Database Schema, API Contracts

4. **Given** Architecture generation
   **When** Document is created
   **Then** Technical decisions from architecture are documented

5. **Given** Documents are generated
   **When** Documents are saved
   **Then** Both documents are saved to project_documents table with doc_type='prd' or 'architecture'

## Tasks / Subtasks

- [ ] Task 1: Create project_documents table schema (AC: #5)
  - [ ] Add migration for project_documents table
  - [ ] Include columns: id, project_id, doc_type, content, version, created_at, updated_at
  - [ ] Add unique constraint on (project_id, doc_type)
  - [ ] Support doc_types: prd, architecture, ux, phases, readme

- [ ] Task 2: Implement PRD generation service (AC: #1, #2)
  - [ ] Create PRDGenerator service
  - [ ] Extract requirements from conversation context
  - [ ] Generate structured PRD markdown
  - [ ] Include all required sections

- [ ] Task 3: Implement Architecture generation service (AC: #3, #4)
  - [ ] Create ArchitectureGenerator service
  - [ ] Extract technical decisions from conversation
  - [ ] Generate structured architecture markdown
  - [ ] Include all required sections

- [ ] Task 4: Create document templates
  - [ ] PRD template with standard sections
  - [ ] Architecture template with standard sections
  - [ ] Placeholder handling for generated content

- [ ] Task 5: Implement document storage (AC: #5)
  - [ ] Create ProjectDocumentRepository
  - [ ] Implement save/update/retrieve methods
  - [ ] Handle versioning (increment on update)
  - [ ] Write to file system as well

- [ ] Task 6: Wire document generation to chat
  - [ ] Detect document generation requests
  - [ ] Trigger appropriate generator
  - [ ] Display generated document preview
  - [ ] Confirm before saving

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain service pattern for document generators
- Repository pattern for document storage
- Template-based document generation

**Document Requirements:**
- Markdown format for all documents
- Versioning for document updates
- Storage in both database and file system

### PRD Template Structure

```markdown
# Product Requirements Document: {project_name}

## Overview
- Vision
- Goals
- Target Users

## Functional Requirements
- FR1: ...
- FR2: ...

## Non-Functional Requirements
- NFR1: ...
- NFR2: ...

## Architecture Overview
- High-level system design

## Database Schema
- Entity relationship overview
```

### Architecture Template Structure

```markdown
# Architecture Document: {project_name}

## System Overview
- System context
- Key components

## Tech Stack
- Backend: ...
- Frontend: ...
- Database: ...

## Database Schema
- Tables and relationships

## API Contracts
- Endpoints and data formats
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/documents/` - Document domain
- `crates/demiarch-core/src/infrastructure/db/project_documents.rs` - Repository
- `crates/demiarch-core/src/application/generators/` - Document generators

**Database Schema:**
```sql
CREATE TABLE project_documents (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    doc_type TEXT NOT NULL CHECK(doc_type IN ('prd', 'architecture', 'ux', 'phases', 'readme')),
    content TEXT NOT NULL,
    version INTEGER DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    UNIQUE(project_id, doc_type)
);
```

### Testing Requirements

- Unit tests for document generators
- Integration tests for document storage
- Template rendering tests
- Conversation extraction tests

### References

- [Source: docs/PRD.md#Document-Auto-Generation] - Document generation requirements
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.4] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
