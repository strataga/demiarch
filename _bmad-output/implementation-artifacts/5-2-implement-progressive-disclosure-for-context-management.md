# Story 5.2: Implement Progressive Disclosure for Context Management

Status: ready-for-dev

## Story

As a user,
I want Demiarch to automatically retrieve only the most relevant context to save tokens,
so that my conversations are efficient and cost-effective.

## Acceptance Criteria

1. **Given** User has a long conversation history and makes a new request
   **When** System prepares context for LLM
   **Then** System retrieves layered summaries from context_summaries table

2. **Given** Summaries are retrieved
   **When** Summary levels are used
   **Then** Summary types follow: detail_level=1 (index), 2 (timeline), 3 (full)

3. **Given** Context is being selected
   **When** Relevance is determined
   **Then** Only relevant summaries are included based on semantic similarity (using sqlite-vec embeddings)

4. **Given** User explicitly requests more detail
   **When** Full context is needed
   **Then** Full context is retrieved only when needed (user explicitly requests more detail)

5. **Given** Context is prepared
   **When** Token count is calculated
   **Then** Total token count is displayed to user

6. **Given** Token savings occur
   **When** User views savings
   **Then** User sees: "Using 3K tokens from conversation (10x savings via progressive disclosure)"

7. **Given** Context would exceed limit
   **When** Summarization is needed
   **Then** If context would exceed limit, system creates automatic summary

## Tasks / Subtasks

- [ ] Task 1: Create context_summaries table schema (AC: #1, #2)
  - [ ] Add migration for context_summaries table
  - [ ] Include columns: id, source_type, source_id, detail_level, content, token_count, parent_summary_id
  - [ ] Add indexes for source and level queries

- [ ] Task 2: Create context_embeddings table schema (AC: #3)
  - [ ] Add migration for context_embeddings table
  - [ ] Include columns: id, summary_id, embedding (vec), embedding_model
  - [ ] Use sqlite-vec for vector storage

- [ ] Task 3: Implement summary generation service (AC: #1, #2, #7)
  - [ ] Create SummaryGenerator service
  - [ ] Generate Level 1 (index) summaries: ~50-100 tokens
  - [ ] Generate Level 2 (timeline) summaries: ~200-500 tokens
  - [ ] Store Level 3 (full) content: ~500-2000 tokens

- [ ] Task 4: Implement embedding generation (AC: #3)
  - [ ] Use text-embedding-3-small model
  - [ ] Generate 1536-dimensional embeddings
  - [ ] Store in context_embeddings table

- [ ] Task 5: Implement smart retrieval (AC: #3, #4)
  - [ ] Create ContextRetriever service
  - [ ] Query embeddings for semantic similarity
  - [ ] Start with Level 1, expand as needed
  - [ ] Respect token budget

- [ ] Task 6: Implement token tracking and display (AC: #5, #6)
  - [ ] Calculate token usage per retrieval
  - [ ] Display savings message to user
  - [ ] Track 10x savings metric

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- 3-layer progressive disclosure (Index/Timeline/Full)
- Vector embeddings for semantic search
- Token-efficient context retrieval (~10x savings)

**Security Requirements:**
- Semantic filter at embedding retrieval to scan for prompt injection patterns

### Summary Levels

```rust
pub enum DetailLevel {
    Index = 1,     // ~50-100 tokens, high-level topics
    Timeline = 2,  // ~200-500 tokens, key events and decisions
    Full = 3,      // ~500-2000 tokens, complete content
}
```

### Retrieval Algorithm

```rust
pub async fn retrieve_context(
    &self,
    query: &str,
    max_tokens: u32,
) -> Result<RetrievedContext> {
    // 1. Always fetch Level 1 (index) for all sources
    let index_summaries = self.fetch_level(DetailLevel::Index)?;

    // 2. Generate query embedding
    let query_embedding = self.generate_embedding(query).await?;

    // 3. Find top-K similar summaries
    let similar = self.knn_search(query_embedding, k=10)?;

    // 4. Expand to Level 2 for top matches if budget allows
    let mut context = index_summaries;
    for item in similar.iter().take(5) {
        if self.fits_budget(context.tokens + item.level2_tokens, max_tokens) {
            context.add(self.fetch_level2(item.source_id)?);
        }
    }

    // 5. Expand to Level 3 for most relevant if budget allows
    if let Some(top) = similar.first() {
        if self.fits_budget(context.tokens + top.level3_tokens, max_tokens) {
            context.add(self.fetch_level3(top.source_id)?);
        }
    }

    Ok(context)
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/context/summary.rs`
- `crates/demiarch-core/src/domain/context/retriever.rs`
- `crates/demiarch-core/src/infrastructure/db/context_summaries.rs`
- `crates/demiarch-core/src/infrastructure/db/context_embeddings.rs`

**Database Schema:**
```sql
CREATE TABLE context_summaries (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,  -- 'conversation', 'feature', 'agent_execution'
    source_id TEXT NOT NULL,
    detail_level INTEGER NOT NULL CHECK(detail_level IN (1, 2, 3)),
    content TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    parent_summary_id TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE context_embeddings (
    id TEXT PRIMARY KEY,
    summary_id TEXT NOT NULL,
    embedding BLOB NOT NULL,  -- sqlite-vec float array
    embedding_model TEXT NOT NULL,
    FOREIGN KEY (summary_id) REFERENCES context_summaries(id)
);
```

### Testing Requirements

- Summary generation tests
- Embedding generation tests
- Retrieval algorithm tests
- Token budget enforcement tests

### References

- [Source: docs/PRD.md#Progressive-Disclosure] - Progressive disclosure requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Progressive-Disclosure] - Architecture details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.2] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
