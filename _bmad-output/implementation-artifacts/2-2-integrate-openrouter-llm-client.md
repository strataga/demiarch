# Story 2.2: Integrate OpenRouter LLM Client

Status: ready-for-dev

## Story

As a user,
I want Demiarch to communicate with LLMs through OpenRouter API,
so that I can use multiple AI models for code generation and conversations.

## Acceptance Criteria

1. **Given** User has configured API key and initiated a chat or code generation request
   **When** Demiarch makes an LLM API call
   **Then** Request is sent to OpenRouter API with proper headers (Authorization: Bearer {encrypted_key})

2. **Given** LLM request is made
   **When** Model selection is needed
   **Then** Model is selectable from configured defaults (anthropic/claude-sonnet-4-20250514)

3. **Given** LLM request is processed
   **When** Building request context
   **Then** Request includes conversation context from chat_messages table

4. **Given** LLM response is received
   **When** Response is processed
   **Then** Response is parsed and saved to chat_messages table with token_count

5. **Given** LLM response is processed
   **When** Cost tracking occurs
   **Then** Cost is calculated and saved to llm_usage table

6. **Given** Network error occurs
   **When** Request fails
   **Then** Network errors trigger exponential backoff retry (max 3 retries, 30s max delay)

7. **Given** Model fails
   **When** Fallback is needed
   **Then** Model fallback activates on failure (default → Haiku → GPT-4o)

## Tasks / Subtasks

- [ ] Task 1: Create LLMClient struct (AC: #1, #2)
  - [ ] Add reqwest 0.12 dependency
  - [ ] Create LLMClient with configuration
  - [ ] Implement OpenRouter API request building
  - [ ] Handle API key decryption from secure storage

- [ ] Task 2: Implement request building (AC: #1, #3)
  - [ ] Create CompletionRequest struct
  - [ ] Build conversation context from chat_messages
  - [ ] Set proper headers (Authorization, Content-Type)
  - [ ] Configure timeout (120s default)

- [ ] Task 3: Implement response parsing (AC: #4, #5)
  - [ ] Create CompletionResponse struct
  - [ ] Parse token counts from response
  - [ ] Extract content and model information
  - [ ] Integrate with CostTracker for cost recording

- [ ] Task 4: Implement retry logic (AC: #6)
  - [ ] Create RetryPolicy struct
  - [ ] Implement exponential backoff (1s → 2s → 4s, max 30s)
  - [ ] Define retryable error types (RateLimit, Timeout, ServerError, NetworkError)
  - [ ] Add max retry count (3)

- [ ] Task 5: Implement model fallback (AC: #7)
  - [ ] Create fallback chain configuration
  - [ ] Implement complete_with_fallback() method
  - [ ] Log fallback usage
  - [ ] Default chain: claude-sonnet-4 → claude-haiku → gpt-4o

- [ ] Task 6: Create LLM domain service
  - [ ] Create LLMService trait
  - [ ] Implement chat completion method
  - [ ] Wire to chat interface

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Domain service pattern for LLM interactions
- Retry with exponential backoff
- Model fallback chain
- Cost tracking integration

**Technology Stack:**
- reqwest 0.12 for HTTP client
- tokio 1.41 for async runtime
- serde for JSON serialization

### OpenRouter API Reference

```rust
// Request structure
POST https://openrouter.ai/api/v1/chat/completions
Headers:
  Authorization: Bearer {API_KEY}
  Content-Type: application/json

// Request body
{
  "model": "anthropic/claude-sonnet-4-20250514",
  "messages": [
    {"role": "user", "content": "..."}
  ]
}

// Response
{
  "choices": [{"message": {"content": "..."}}],
  "usage": {"prompt_tokens": N, "completion_tokens": N}
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/llm/` - LLM domain module
- `crates/demiarch-core/src/infrastructure/llm/` - LLM client implementation
- `crates/demiarch-core/src/infrastructure/llm/openrouter.rs` - OpenRouter client

**Error Handling:**
```rust
pub enum LLMError {
    NetworkError(reqwest::Error),
    RateLimited(u64),
    ServerError(String),
    Timeout,
    AllModelsFailed,
}
```

### Testing Requirements

- Unit tests for request building
- Mock server tests for response parsing
- Retry logic tests
- Fallback chain tests

### References

- [Source: docs/PRD.md#LLM-Integration] - LLM client requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Network-Resilience] - Retry and fallback patterns
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
