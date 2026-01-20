# Story: Set up SQLite Database per Project

**Epic:** Foundation & Project Setup  
**Priority:** High  
**Status:** ready-for-dev

## User Story
As a developer using Demiarch, I want each project to have its own SQLite database so that project-specific data is isolated and organized.

## Acceptance Criteria
- [ ] Create a SQLite database file for each project
- [ ] Implement database connection pooling for performance
- [ ] Define the initial database schema with all required tables
- [ ] Implement database migrations for schema evolution
- [ ] Add database backup and recovery functionality
- [ ] Ensure database operations are properly error-handled
- [ ] Create database utilities for common operations

## Technical Details

### Database Schema
The database should include tables for:
- `projects` - Project metadata and configuration
- `conversations` - Chat/conversation history
- `agents` - Agent configurations and states  
- `skills` - Learned skills and capabilities
- `code_generation` - Generated code and artifacts
- `llm_calls` - LLM API call logs for cost tracking
- `checkpoints` - Recovery checkpoints
- `plugins` - Plugin configurations and state
- `sessions` - User session management

### Database Location
- Store databases in `~/.demiarch/projects/{project_id}/database.sqlite`
- Create project directory structure if it doesn't exist
- Implement database file permissions (600 - user read/write only)

### Migration System
- Use `sqlx` migration system
- Migrations should be versioned and idempotent
- Include both up and down migration scripts
- Run migrations automatically on application startup

## Implementation Notes
- Use the existing `demiarch-core` infrastructure layer database module
- Leverage the existing repository pattern interfaces
- Implement proper connection pooling configuration
- Add database health checks
- Include database metrics and logging

## Dependencies
- `sqlx` for SQLite operations
- `rusqlite` for direct SQLite access when needed
- `uuid` for generating unique identifiers
- `chrono` for timestamps
- `serde` for serialization

## Testing
- [ ] Unit tests for database operations
- [ ] Integration tests for migration system
- [ ] Performance tests for connection pooling
- [ ] Tests for error scenarios (corrupt DB, permission issues)
- [ ] Tests for concurrent database access

## Definition of Done
- [ ] All acceptance criteria met
- [ ] Database schema defined and migrations working
- [ ] All database operations properly tested
- [ ] Documentation updated
- [ ] Code review completed
- [ ] Integration with existing codebase verified