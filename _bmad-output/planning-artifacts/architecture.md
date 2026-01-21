---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments: ["docs/PRD.md"]
workflowType: 'architecture'
project_name: 'demiarch'
user_name: 'Jason'
date: '2026-01-19'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**

- **AI-Powered Code Generation**: Conversational discovery interface that generates code into user's local repositories, supporting all major web, mobile, desktop, and backend frameworks
- **Document Auto-Generation**: Automated generation of PRD, Architecture, and UX design documents through AI collaboration
- **Phase Planning**: Breakdown of projects into phases with user stories, acceptance criteria, and implementation tracking
- **Multi-Project Concurrency**: Support for working on 3-5 projects simultaneously with cross-project context sharing and resource locking
- **Russian Doll Agent System**: 3-level hierarchical agent architecture (Orchestrator → Planner → Coder/Reviewer/Tester) with autonomous delegation
- **Progressive Disclosure**: Token-efficient context retrieval with 3-layer summaries (Index/Timeline/Full) achieving ~10x token savings
- **Learned Skills Extraction**: Autonomous extraction of debugging knowledge as reusable skills with embedding-based semantic search and RL quality optimization
- **Dynamic Model Routing**: RL-optimized selection between specialized (Codestral, Qwen-Math) and generalist (Claude, GPT-4o) models based on task type and learned performance
- **Conflict Resolution**: Smart detection and handling of user edits vs AI-regenerated code with manual merge support
- **Plugin System**: Extensible framework for code generation with WASM sandboxing, offline license verification (ed25519), and pricing models (free, paid, usage, trial, watermark)
- **Storage Pattern**: SQLite primary storage with JSONL git-sync format for explicit, non-automatic sync operations
- **Recovery System**: Checkpoint-based rollback for safe experimentation with generation transactions and cleanup tracking
- **Lifecycle Hooks**: Extensible event handling for session_start, session_end, pre_generation, post_generation, on_error, on_checkpoint
- **Multi-Interface Support**: CLI (TUI with ratatui), GUI (Tauri + React), and future REST API using shared command library pattern
- **Cost Management**: Real-time cost tracking with budget enforcement, daily limits, and cost alerts
- **Offline Support**: Degraded mode with operation queuing and automatic retry when connectivity restored
- **Security Infrastructure**: AES-GCM encryption for API keys with argon2 key derivation, audit logging, update signature verification, semantic filtering for prompt injection prevention

**Non-Functional Requirements:**

- **Local-First Design**: All user data stored in SQLite database, no cloud accounts required, no telemetry by default
- **Security**:
  - API keys encrypted at rest using AES-GCM with machine-based key derivation (argon2)
  - OS keyring storage preferred, encrypted SQLite fallback with zeroize for memory hardening
  - Plugin sandboxing using WASM with CPU/memory limits and multi-tier verification
  - Offline license verification with ed25519 cryptographic signatures
  - Comprehensive audit logging for security events
  - Rate limiting (requests/minute, tokens/minute)
  - Regular security key rotation
  - Semantic filter at embedding retrieval to prevent indirect prompt injection
  - Checkpoint file signing with ed25519 for integrity verification
  - Plugin signing ecosystem with three-tier trust model (built-in, verified, unverified)
- **Performance**:
  - Progressive disclosure reduces token usage by ~10x through layered context summaries
  - Vector-based semantic search for skills and context (1536-dimensional embeddings)
  - Efficient conflict detection using file checksums
  - Caching support for LLM prompts (prompt caching)
- **Reliability**:
  - Network resilience with exponential backoff retry (max 3 retries, 30s max delay)
  - Automatic model fallback (default → Haiku → GPT-4o)
  - Database migrations for schema evolution
  - Recovery system with checkpoint-based rollback
  - Generation transaction tracking for cleanup on failure
- **Scalability**:
  - Multi-project resource locking using async semaphores with timeouts
  - Cross-project search and context sharing (opt-in only)
  - Session management with context window tracking
  - Plugin registry with hot-swappable capabilities
- **Usability**:
  - Multiple interfaces (CLI, GUI, API) sharing same command library
  - Explicit control (no automatic git operations, no background processes)
  - Clear error messages with actionable suggestions
  - Cost dashboard with per-model and per-feature breakdown
- **Extensibility**:
  - Plugin system for framework-specific code generation
  - Lifecycle hooks for custom behavior at key events
  - Configurable model routing rules with RL optimization
  - Script-based hook handlers for external automation

**Scale & Complexity:**

- Primary domain: Full-stack AI application (Rust backend + React/TypeScript frontend + WASM plugins)
- Complexity level: **High/Enterprise**
- Estimated architectural components: 15+ major subsystems (CLI, GUI, Agent System, LLM Client, Cost Tracker, Recovery Manager, Plugin Manager, Conflict Resolution, Skill Extraction, Context Manager, Model Router, Security, Storage, Sync, Hooks, Multi-Project Coordinator)

### Technical Constraints & Dependencies

**External Dependencies:**
- **OpenRouter API**: Required for LLM access (Claude, GPT-4o, specialized models)
- **Vector Extension**: SQLite requires sqlite-vss or sqlite-vec for embedding-based search
- **WASM Runtime**: Wasmtime for plugin sandboxing with CPU/memory limits
- **Git Integration**: git2-rs for repository operations (manual sync, not automatic)
- **Cryptography**: ed25519-dalek (signatures), aes-gcm (encryption), argon2 (key derivation), ring (verification), rand_chacha (nonce generation), zeroize (memory hardening)
- **Keyring**: keyring crate for OS-integrated credential storage (preferred)

**Technology Stack (from PRD):**
- **Backend**: Rust 1.41+ with Tokio async runtime
- **Frontend**: React 18.3+ + TypeScript 5.6.0 + Tauri 2.1
- **Database**: SQLite with rusqlite for extension loading, sqlx for async queries
- **Security**: seccomp for Linux sandboxing, network namespaces for network isolation
- **Rate Limiting**: Governor crate for request/token rate limiting
- **Caching**: lru and moka for async cache
- **CLI**: ratatui 0.29 for TUI
- **Serialization**: serde, serde_json
- **Security Scanning**: cargo-audit for Rust, npm audit for frontend dependencies

**Constraints:**
- All data must be local (SQLite) with git-friendly export (JSONL)
- No automatic background processes or sync operations
- User owns all generated code and can edit freely
- AI must respect user edits during regeneration
- Maximum 3-5 concurrent projects with resource locking
- Plugin execution must be sandboxed (WASM) with CPU/memory limits
- API keys must be encrypted at rest (AES-GCM with argon2) with OS keyring preference
- License verification must work offline (ed25519 signatures)
- No telemetry or data collection by default
- Multi-platform support (Linux, macOS, Windows)
- Embedding retrieval must be filtered for prompt injection patterns
- All checkpoints must be signed and verified before restore
- Plugins must be signed and verified for tier 2 (verified third-party) status
- SQLite extensions only loaded from signed paths with signature verification

### Cross-Cutting Concerns Identified

**1. Agent Coordination & Delegation**
- 3-level hierarchy with parent-child relationships
- Lock management for shared resources (files, database, LLM, features) with 5-minute timeouts
- Session management with context window tracking
- Cross-project search and context sharing (opt-in only, explicit project linking)
- Each agent runs in separate Tokio task with independent supervision trees

**2. Conflict Detection & Resolution**
- File checksum tracking for detecting user edits
- Conflict status states (none, pending, resolved_keep_user, resolved_keep_ai, resolved_merged)
- Manual merge support in GUI with security annotations
- Rollback capability through generation transactions

**3. Plugin Security & Extensibility**
- WASM sandboxing with fuel-based CPU limiting (10M fuel max, randomized to prevent side-channel timing attacks)
- Multi-tier verification: (1) Built-in plugins (trusted), (2) Verified third-party (signed with ed25519), (3) Unverified (sandboxed only)
- Capability checks at WASM host boundary, not within plugin
- Capabilities-based permission system (read_files, write_files, network, execute_commands)
- Offline license verification with ed25519 signatures
- Pricing models (free, paid, usage, trial, watermark)
- Plugin execution logging with fuel consumption metrics, flag outliers

**4. Cost Management**
- Real-time cost tracking per model, per feature, per project
- Budget enforcement with daily limits and cost alerts
- Token-efficient context retrieval (~10x savings via progressive disclosure)
- Model routing optimization (specialized vs generalist)

**5. Progressive Disclosure & Context Management**
- 3-layer summaries (Index/Timeline/Full) for token efficiency
- Vector embeddings for semantic search
- Detail level selection based on relevance
- Automatic summarization when context exceeds limits
- **Security**: Semantic filter at embedding retrieval to scan for prompt injection patterns (~50ms overhead)
- **Architectural Decision**: Show "context summary" visualization to let users see what's being retrieved at each layer (balances token efficiency with transparency)

**6. Model Routing & RL Optimization**
- Task type classification (code_generation, debugging, planning, math, etc.)
- Specialized model registry (Codestral for code, Qwen-Math for math)
- Performance tracking (quality, latency, cost, success rate)
- RL feedback loop for model selection optimization

**7. Recovery & Rollback**
- Checkpoint-based state snapshots with ed25519 signing
- Generation transaction tracking (files created/modified)
- Automatic cleanup on failure
- Restore capability with conflict handling and signature verification

**8. Lifecycle Hooks**
- Extensible event system (session_start, pre_generation, post_generation, on_error, on_checkpoint)
- Handler types: internal, plugin, script
- Execution tracking (result, duration, output)

**9. Security & Audit**
- AES-GCM encryption for API keys with argon2 key derivation
- OS keyring storage preferred (keyring crate), encrypted SQLite fallback with zeroize
- Cryptographically secure nonce generation (ChaChaRng from rand_chacha) - never reused
- Ed25519 signature verification for updates, licenses, checkpoints, and plugins
- Comprehensive audit logging with retention policy
- Regular key rotation
- No telemetry or data collection by default
- Semantic filter at embedding retrieval for prompt injection prevention
- Multi-layer plugin security: WASM + fuel limits + seccomp + network namespaces

**10. Multi-Project Resource Management**
- Resource locks using async semaphores with 5-minute timeouts, forced release on deadlock
- Cross-project reference tracking (opt-in only)
- Session switching with context preservation
- Concurrent project limits (3-5 projects)
- **Security**: SQLite database per project (not shared) for stronger isolation
- **Architectural Decision**: Design explicit "session contexts" with visual indicators of active project and context window (balances multi-project complexity with usability)

**11. Offline Support**
- Degraded mode detection
- Operation queuing with retry logic
- Network monitoring and auto-recovery
- Graceful degradation of features

**12. CLI as Library Pattern**
- All commands as reusable library functions
- Shared between CLI (TUI), GUI (Tauri), and future API
- Consistent error handling and user feedback

### Critical Security Considerations

**1. Progression Disclosure Prompt Injection Vulnerability**
- **Threat**: Attacker crafts malicious code with specific content that, when embedded and retrieved, causes LLM to follow hidden instructions
- **Mitigation**: Semantic filter at embedding retrieval scans for prompt injection patterns ("ignore previous instructions", "new role:", etc.)
- **Trade-off**: Adds ~50ms latency per retrieval but prevents indirect prompt injection
- **Implementation**: Pre-retrieval filter checks all retrieved context before passing to LLM

**2. API Key Protection**
- **Threat**: Nonce reuse could leak encryption keys (catastrophic), memory dumps could recover keys
- **Mitigation**:
  - Cryptographically secure nonce generation (ChaChaRng)
  - OS keyring storage preferred, encrypted SQLite fallback
  - Zero memory containing keys immediately after use (zeroize crate)
- **Trade-off**: OS keyring depends on platform availability, encrypted config always works
- **Implementation**: Multi-layer approach with zeroize memory hardening

**3. Plugin System Security**
- **Threat**: WASM sandbox escape, capability bypass, license verification bypass, side-channel attacks via fuel consumption
- **Mitigation**:
  - Multi-tier sandboxing: WASM + fuel limits + seccomp + network namespaces
  - Capability checks at host boundary, randomized fuel limits (prevent side-channel timing)
  - License keys burned into binary (not loaded from disk)
  - Three-tier plugin trust model (built-in, verified, unverified)
- **Trade-off**: More complex but enables trusted ecosystem
- **Implementation**: All plugin executions logged, flag fuel consumption outliers

**4. Multi-Project Isolation**
- **Threat**: Resource lock poisoning, cross-project context injection, session hijacking, deadlock amplification
- **Mitigation**:
  - SQLite database per project (stronger isolation than schema prefixes)
  - Locks with timeouts + forced release on deadlock
  - Cross-project search: opt-in only, explicit project linking
  - Session tokens with short expiry, rotation on project switch
- **Trade-off**: More files but stronger isolation, easier backup/migration
- **Implementation**: Lock contention monitoring, cross-project reference spike detection

**5. Agent System Resilience**
- **Threat**: Parent crash cascades, prompt injection via user-edited files, skill system poisoning, LLM credential theft
- **Mitigation**:
  - Each agent runs in separate Tokio task with independent supervision trees
  - User-edited files scanned for suspicious patterns (eval(), exec(), suspicious imports) before AI consumption
  - Skills require manual approval or rate-limit with RL feedback
  - LLM credentials redacted from error messages and logs
- **Trade-off**: Overhead of file scanning vs security
- **Implementation**: Track agent crash rates, skill success rates, prompt injection attempts

**6. Update & Supply Chain Security**
- **Threat**: Signature verification bypass, MITM on plugin installation, dependency compromise
- **Mitigation**:
  - Ed25519 public keys compiled into binary (const arrays), not loaded from files
  - Cargo.lock and package-lock.json committed, verify checksums
  - Plugin signature verification: verify BOTH license and plugin signatures
  - Automated dependency security scanning (cargo-audit, npm audit)
- **Trade-off**: Compile-time keys reduce flexibility, automated scanning adds CI overhead
- **Implementation**: Alert on signature failures, automated vulnerability announcements

**7. File System & Git Integrity**
- **Threat**: JSONL injection, malicious merge acceptance, checkpoint tampering, git credential theft
- **Mitigation**:
  - JSONL sync: validate each entry against schema before writing
  - Conflict resolution: clear diff with security annotations (e.g., "adds exec()")
  - Checkpoint signing: sign with ed25519, verify on restore
  - Git operations: use libgit2's credential helpers, never store credentials directly
- **Trade-off**: Validation adds processing time
- **Implementation**: Log all git operations, flag suspicious file modifications

**8. Database & SQLite Security**
- **Threat**: Extension loading attacks, SQL injection via plugins, privilege escalation
- **Mitigation**:
  - SQLite extensions: only load from signed paths, verify signatures
  - All queries use sqlx compile-time checked queries or strict parameterization
  - Database files: restrictive permissions (0600), stored in secure directory
  - JSONL import: validate against JSON schema before database import
- **Trade-off**: Extension signing limits extensibility
- **Implementation**: Log all extension load attempts, detect SQL injection patterns

### Critical Trade-Offs and Tensions

**1. Power User vs New User Balance**
- Product wants power features (3-5 concurrent projects, Russian Doll agents)
- UX wants onboarding simplicity and clear mental models
- **Resolution**: Design progressive complexity disclosure - core features simple, power features discoverable
- **Architectural Implication**: Modular component design with feature flags for progressive enhancement

**2. Explicit Control vs Developer Flow**
- Product insists on no auto-operations (user owns everything)
- UX wants muscle memory compatibility (auto-sync, background processes)
- **Resolution**: Keep explicit control but provide CLI shortcuts and GUI workflows that feel fluid
- **Architectural Implication**: Command library must support both explicit sync and batch operations with user confirmation

**3. Token Efficiency vs Transparency**
- Architecture achieves 10x savings through progressive disclosure
- UX worries about "hidden context" feeling opaque
- **Resolution**: Show "context summary" visualization - let users see what's being retrieved at each layer
- **Architectural Implication**: Progressive disclosure system must include UI layer for context visualization (Index/Timeline/Full view)

**4. Token Efficiency vs Security**
- Progressive disclosure with embeddings could enable indirect prompt injection
- Security requires semantic filtering at retrieval time
- **Resolution**: Implement semantic filter for prompt injection patterns (~50ms overhead)
- **Architectural Implication**: Context manager must integrate security filter at retrieval pipeline, pre-retrieval validation

**5. Multi-Project Complexity vs Usability**
- Product demands 3-5 concurrent projects for productivity
- Architecture requires complex locking and session management
- UX needs seamless switching with context preservation
- **Resolution**: Design explicit "session contexts" with visual indicators of active project and context window
- **Architectural Implication**: Session manager must track context windows per project and provide quick-switch UI/API patterns

**6. Multi-Project Complexity vs Security Isolation**
- Product wants cross-project context sharing for productivity
- Security wants strong isolation to prevent attacks between projects
- **Resolution**: SQLite database per project + opt-in cross-project search with explicit linking
- **Architectural Implication**: Strong isolation by default, sharing by explicit user action

**7. Plugin Security vs Capability**
- Architecture enforces WASM sandboxing for security
- Product needs extensible framework for third-party plugins
- UX requires clear permissions/capabilities visualization
- **Resolution**: Tiered permission model with clear UI showing what capabilities a plugin requests (like mobile app permissions)
- **Architectural Implication**: Plugin capability system must be declarative, auditable, and surfaced to UI for approval before first use

**8. Plugin Security vs Ecosystem Growth**
- Three-tier trust model (built-in, verified, unverified) provides strong security
- May limit plugin adoption if verification process is too strict
- **Resolution**: Build verification tooling and community trust framework, allow gradual migration from unverified to verified
- **Architectural Implication**: Plugin signing infrastructure must support automated verification pipelines

**9. Agent Hierarchy Complexity vs Reliability**
- Russian Doll agents enable powerful task decomposition
- 3-level hierarchy adds failure modes (parent crashes kill children)
- **Resolution**: Implement graceful degradation and checkpoint recovery at each agent level
- **Architectural Implication**: Each agent type (Orchestrator, Planner, Coder, Reviewer, Tester) needs independent state management and parent-child recovery protocols

**10. Checkpoint Integrity vs Performance**
- Signing all checkpoints with ed25519 provides integrity verification
- Signing adds overhead and requires key management
- **Resolution**: Sign checkpoints, use user's signing key, verify on restore
- **Architectural Implication**: Recovery system must integrate signing at checkpoint creation, verification at restore time

**11. API Key Storage: OS Keyring vs Encrypted Config**
- OS keyring is most secure but depends on platform availability
- Encrypted config always works but less secure than OS keyring
- **Resolution**: Try OS keyring first (keyring crate), fall back to encrypted SQLite with zeroize
- **Architectural Implication**: Key storage must be abstracted behind trait to support multiple backends

**12. Supply Chain Security vs Developer Experience**
- Dependency pinning and automated scanning improve security
- Adds CI/CD overhead and complexity to onboarding
- **Resolution**: Mandatory security scanning in CI, cargo-audit and npm audit automated
- **Architectural Implication**: CI/CD pipeline must integrate security scanning with failure on high-severity vulnerabilities

## Starter Template Evaluation

### Primary Technology Domain

**Complex Full-Stack AI Desktop Application** - Demiarch is a highly specialized full-stack system combining:
- Rust backend with advanced async programming (Tokio)
- Tauri GUI with React + TypeScript
- CLI component with terminal UI (ratatui)
- WASM plugin sandboxing system
- Complex agent orchestration and AI integration

This complexity exceeds typical starter templates which target simple applications.

### Starter Options Considered

**1. Official Tauri Starter (create-tauri-app)**
- **Advantages**: Official, well-maintained, cross-platform, Tauri 2.x ready
- **Limitations**: Doesn't provide CLI component, agent system, plugin architecture, or security infrastructure
- **Suitability**: Foundation layer only - provides Rust + Tauri + React + TypeScript base

**2. Rust CLI Framework Starters (ratatui, crossterm)**
- **Advantages**: Terminal UI scaffolding
- **Limitations**: No GUI, no Tauri integration, no multi-app architecture
- **Suitability**: Not applicable - needs full GUI + CLI integration

**3. Full-Stack Rust Starters (Actix, Axum, Tauri plugins)**
- **Advantages**: Backend-heavy, API-focused
- **Limitations**: No desktop GUI, no CLI, no AI-specific infrastructure
- **Suitability**: Not applicable - desktop-first architecture

**4. Custom Build (No Starter)**
- **Advantages**: Complete control, optimized for specific requirements
- **Limitations**: High upfront effort, reinventing wheels for basic setup
- **Suitability**: Suboptimal - loses Tauri's cross-platform benefits

### Selected Starter: create-tauri-app (React + TypeScript)

**Rationale for Selection:**

1. **Foundation Alignment**: Provides exact technology stack specified in PRD (Rust backend, Tauri 2.x, React 18+, TypeScript 5.6)
2. **Security Audited**: Tauri undergoes security audits for releases, aligning with demiarch's security-first approach
3. **Cross-Platform**: Native support for Linux, macOS, Windows with automated build pipelines
4. **Extensibility**: Tauri plugin system can be leveraged for custom extensions
5. **Proven Stability**: 102k+ GitHub stars, active maintenance, extensive documentation
6. **Build Pipeline**: Integrated bundling for .app, .dmg, .exe installers, reducing DevOps overhead

**Custom Architecture Required:**
- CLI crate with ratatui (separate binary or workspace crate)
- SQLite with vector extensions (sqlite-vss or sqlite-vec)
- Agent orchestration framework (3-level Russian Doll hierarchy)
- WASM plugin infrastructure with multi-tier verification
- Progressive disclosure system with embedding support
- Security layer (encryption, signing, keyring integration)
- Cost management and budget enforcement
- Recovery system with checkpointing
- Conflict resolution with manual merge
- Model routing with RL optimization
- 15+ additional subsystems identified in architecture

**Initialization Command:**

```bash
npm create tauri-app@latest demiarch --template react-ts
```

**Architectural Decisions Provided by Starter:**

**Language & Runtime:**
- **Rust 1.41+** (matches PRD specification)
- **Tokio async runtime** for Rust backend (matches PRD specification)
- **TypeScript 5.6.0** for frontend (matches PRD specification)
- **JavaScript interop** via Tauri's `invoke` system for Rust ↔ frontend communication

**Styling Solution:**
- **Base CSS** - Tauri starter provides basic CSS foundation
- **Note**: Demiarch will likely need additional UI libraries (Tailwind, styled-components, or CSS modules) based on UX requirements

**Build Tooling:**
- **Vite** for frontend development and bundling
- **Cargo** for Rust compilation and workspace management
- **Tauri CLI** for cross-platform app bundling (.app, .dmg, .exe, .AppImage, etc.)
- **Hot reloading** for both Rust (via Tauri dev watcher) and React (via Vite HMR)

**Testing Framework:**
- **Jest/Vitest** typically included for frontend testing
- **No Rust testing framework** - demiarch will need to add cargo test framework
- **No integration testing setup** - demiarch will need end-to-end testing infrastructure

**Code Organization:**
- **src-tauri/** - Rust backend code (main.rs, lib.rs)
- **src/** - React frontend code (App.tsx, main.tsx)
- **public/** - Static assets
- **Cargo.toml** - Rust dependencies (demiarch will add 15+ crates)
- **package.json** - Node.js dependencies for frontend

**Development Experience:**
- **Hot reload** - Both Rust and React refresh on file changes
- **TypeScript** - Type safety across Rust ↔ TypeScript boundary via Tauri's type generation
- **Dev server** - Local development with Tauri + Vite integration
- **Build commands** - `cargo tauri dev`, `cargo tauri build`, `cargo tauri bundle`

**Note:** Project initialization using this command should be first implementation story, followed by incremental architecture layer construction (CLI crate, SQLite schema, agent system, plugin infrastructure, security layer, etc.).

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**

1. **SQLite Vector Extension**: sqlite-vec (v0.1.6) - Active development with Mozilla Builders sponsorship
2. **Project Structure**: Cargo Workspace with 3 Crates (demiarch-core, demiarch-cli, demiarch-gui)
3. **UI Framework**: Tailwind CSS (v3.4) + shadcn/ui (Radix UI components)
4. **State Management**: Zustand (v5.0.0) - Lightweight, providerless pattern
5. **Architectural Patterns**: DDD + SOLID principles with trait-based design

**Important Decisions (Shape Architecture):**
- CLI as Library Pattern: Shared crate with command traits
- Testing Strategy: Needs to be defined (deferred to patterns step)
- Error Handling: Needs to be defined (deferred to patterns step)

**Deferred Decisions (Post-MVP):**
- Internationalization (i18n)
- Theming (dark/light mode)
- Accessibility compliance level (WCAG)

### Data Architecture

**SQLite Vector Extension: sqlite-vec (v0.1.6)**

- **Decision**: Choose sqlite-vec over sqlite-vss (abandoned)
- **Version**: v0.1.6 (Nov 20, 2024)
- **Rationale**: Active development, 3x community engagement (6.7k stars vs 2k), Mozilla Builders sponsorship, pure C (no dependencies), portability (WASM, Raspberry Pi), rich features (metadata, partitions, auxiliary columns)
- **Affects**: Progressive disclosure system, learned skills subsystem, context manager
- **Provided by Starter**: No - must be added as Cargo dependency: `cargo add sqlite-vec`
- **APIs Used**: `vec0` virtual tables, KNN queries with `match` operator, metadata/auxiliary column support

**Database Schema Strategy:**

- **Decision**: SQLite per project (not shared) + JSONL for git sync
- **Rationale**: Strong security isolation, easier backup/migration, aligns with multi-project security requirements
- **Affects**: Multi-project coordinator, storage layer, sync system
- **Provided by Starter**: Partial - Tauri starter provides SQLite setup but not per-project database pattern

**Vector Embedding Storage:**

- **Decision**: Store 1536-dimensional embeddings as float[] in sqlite-vec
- **Rationale**: Matches OpenAI/Claude embedding dimensions, enables KNN search for progressive disclosure
- **Affects**: Progressive disclosure, learned skills, context retrieval
- **Provided by Starter**: No

### Authentication & Security

**API Key Storage:**

- **Decision**: OS keyring (keyring crate) preferred, encrypted SQLite fallback with zeroize
- **Version**: keyring crate (latest stable)
- **Rationale**: Most secure (OS-integrated), fallback ensures cross-platform compatibility, zeroize for memory hardening
- **Affects**: Security domain, configuration management
- **Provided by Starter**: No - custom implementation required

**Encryption Strategy:**

- **Decision**: AES-GCM for encryption, argon2 for key derivation, ChaChaRng for nonce generation
- **Version**: aes-gcm crate, argon2 crate, rand_chacha crate
- **Rationale**: Authenticated encryption (AES-GCM), memory-hard KDF (argon2), cryptographically secure RNG (ChaChaRng)
- **Affects**: API key storage, plugin licensing, checkpoint signing
- **Provided by Starter**: No - custom implementation required

### API & Communication Patterns

**Rust ↔ TypeScript Communication:**

- **Decision**: Tauri's `invoke` system with auto-generated TypeScript types
- **Rationale**: Type-safe interop, automatic type generation, Tauri framework native
- **Affects**: All CLI/GUI shared commands, error propagation
- **Provided by Starter**: Yes - Tauri provides invoke system and type generation

**Error Handling Standards:**

- **Decision**: Defer to patterns step - needs DDD + SOLID application
- **Rationale**: Error types should be defined at domain layer with application service error handling
- **Affects**: All modules, testing, user feedback
- **Provided by Starter**: No

### Frontend Architecture

**UI Framework: Tailwind CSS (v3.4) + shadcn/ui**

- **Decision**: Tailwind CSS + shadcn/ui (Radix UI components)
- **Version**: Tailwind 3.4, shadcn/ui (latest)
- **Rationale**: Rapid UI development for complex dashboards, no runtime overhead (copy-paste components), excellent TypeScript support, smaller bundle size, easy dark/light mode theming
- **Affects**: GUI (demiarch-gui), bundle size, development speed
- **Provided by Starter**: No - must be configured in Vite + Tauri

**State Management: Zustand (v5.0.0)**

- **Decision**: Zustand for React state management
- **Version**: v5.0.0 (matches PRD specification)
- **Rationale**: Lightweight (1KB gzipped), no provider wrapping, excellent TypeScript support, fits Tauri's command pattern, small bundle size
- **Affects**: GUI state (projects, agents, costs, conflicts), CLI/GUI sharing patterns
- **Provided by Starter**: No - must be added: `npm install zustand`

### Infrastructure & Deployment

**Project Structure: Cargo Workspace with 3 Crates**

- **Decision**: Workspace structure with demiarch-core, demiarch-cli, demiarch-gui
- **Rationale**: Matches Rust best practices, Tauri uses workspace pattern, enables shared types and dependencies, CLI and GUI can share demiarch-core library
- **Affects**: Build configuration, dependency management, CLI/GUI shared library pattern
- **Provided by Starter**: Partial - Tauri provides src-tauri/ crate structure, we extend with workspace pattern

**Architectural Patterns: DDD + SOLID**

- **Decision**: Apply Domain-Driven Design and SOLID principles throughout codebase
- **Rationale**: DDD separates business logic from infrastructure, SOLID ensures maintainable/testable code, both align with Rust's type system and trait-based design
- **Affects**: Code organization, testing strategy, error handling, extensibility
- **Provided by Starter**: No - custom architecture pattern to implement

**Core Domains (DDD Structure):**

1. **Projects Domain** - Project creation, management, context windows
2. **Agents Domain** - Orchestration, delegation, agent lifecycle
3. **Context Domain** - Progressive disclosure, embeddings, retrieval
4. **Skills Domain** - Extraction, storage, semantic search, RL optimization
5. **Code Generation Domain** - File operations, conflict detection, rollback
6. **Plugin Domain** - Loading, sandboxing, capability checks, licensing
7. **Cost Domain** - Tracking, budgeting, alerts, model routing
8. **Security Domain** - Encryption, signing, audit logging
9. **Recovery Domain** - Checkpoints, transactions, cleanup

**SOLID Principles Applied:**

1. **Single Responsibility** - Each domain has one reason to change
2. **Open/Closed** - Plugin system enables extension without modifying core
3. **Liskov Substitution** - Command traits allow swapping implementations (CLI/GUI/API)
4. **Interface Segregation** - Domain-specific traits (ProjectRepository, AgentOrchestrator, CostTracker)
5. **Dependency Inversion** - Core depends on abstractions (traits), not concrete implementations

**DDD Directory Structure:**

```
demiarch-core/
├── domain/
│   ├── projects/         // Project entities, value objects, repository traits
│   ├── agents/           // Agent types, delegation logic
│   ├── context/          // Context retrieval, embeddings
│   ├── skills/           // Skill extraction, quality metrics
│   ├── code_generation/   // File operations, conflicts, rollback
│   ├── plugins/          // Plugin loading, sandboxing, licensing
│   ├── cost/             // Tracking, budgeting, alerts, model routing
│   ├── security/         // Encryption, signing, audit logging
│   └── recovery/         // Checkpoints, transactions, cleanup
├── application/          // Use cases/orchestrators (DDD application services)
│   ├── generate_code.rs
│   ├── create_project.rs
│   ├── manage_session.rs
│   └── ...
├── infrastructure/       // SQLite, LLM client, git integration, WASM runtime
│   ├── db/
│   ├── llm/
│   ├── git/
│   └── wasm/
└── interfaces/           // Traits for external dependencies
```

### Decision Impact Analysis

**Implementation Sequence:**

1. **Foundation**: Cargo workspace setup, Tauri initialization with create-tauri-app
2. **Core Infrastructure**: SQLite database with sqlite-vec, error handling patterns
3. **Domain Layers**: Implement core domains with DDD structure (Projects, Security, Recovery)
4. **Application Services**: Orchestration logic across domains (code generation use case)
5. **CLI Crate**: Implement CLI with ratatui using shared command traits
6. **GUI Crate**: Implement GUI with Tauri + React + Tailwind + shadcn/ui + Zustand
7. **Advanced Features**: Agent system, progressive disclosure, plugin system

**Cross-Component Dependencies:**

- **sqlite-vec → Progressive disclosure + learned skills + context retrieval**
- **DDD domains → CLI + GUI shared command traits**
- **SOLID traits → Testing strategy (mock traits for unit tests)**
- **Zustand → Tauri invoke integration for state sync**
- **shadcn/ui → Tailwind CSS configuration + theming**
- **OS keyring → Encrypted SQLite fallback for backup**

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:**

8 major categories where AI agents could make different choices leading to implementation conflicts:
1. Naming Conventions (Database, API, Code)
2. Project Organization (Tests location, utilities placement, structure)
3. Format Standards (API responses, data formats)
4. Communication Patterns (Events, state management)
5. Process Patterns (Error handling, loading states)
6. Database Patterns (Migration strategy, transaction handling)
7. File Organization (Config, assets, docs)
8. Testing Patterns (Unit vs integration, test naming)

### Naming Patterns

**Database Naming Conventions:**

**Rules:**
- All tables: snake_case, lowercase, plural nouns
- All columns: snake_case, lowercase
- Foreign keys: `{table}_id` format
- Indexes: `idx_{table}_{column}` format
- Primary keys: `id` column (auto-increment integer or UUID)

**Examples:**
```sql
-- Tables
projects, agents, sessions, skills, embeddings, checkpoints, conflicts, costs

-- Columns
project_id, agent_id, created_at, updated_at, embedding_vector

-- Foreign Keys
agent_id (references agents.id), project_id (references projects.id)

-- Indexes
idx_projects_name, idx_embeddings_project_id
```

**API Naming Conventions:**

**Rules:**
- Tauri commands: snake_case
- Event names: snake_case, past tense
- State fields: camelCase (React/TypeScript)
- Config keys: snake_case

**Examples:**
```rust
// Tauri commands (Rust)
#[tauri::command]
async fn generate_code(project_id: String) -> Result<String, String>

// Events (Rust/TypeScript)
ProjectCreated, SessionStarted, SkillExtracted

// State (React/TypeScript - Zustand)
interface AppState {
  activeProject: Project | null
  isGenerating: boolean
  agentStatus: AgentState
}
```

**Code Naming Conventions:**

**Rules:**
- Rust: snake_case for functions/vars, PascalCase for types
- TypeScript/React: camelCase for functions/vars, PascalCase for types/components
- File names: PascalCase for components, snake_case for utilities

**Examples:**
```rust
// Rust
struct Project { id: u32, name: String }
fn create_project(name: &str) -> Result<Project>
impl ProjectRepository for SqliteProjectRepository

// TypeScript/React
interface Project { id: number; name: string }
function createProject(name: string): Promise<Project>
class ProjectCard extends Component { }
```

### Structure Patterns

**Project Organization Rules:**

**Rules:**
- Tests: `tests/` directory for unit tests, `tests/` Rust module for integration tests
- Shared utilities: `common/` crate for cross-crate shared code
- DDD structure: Match domain/ → application/ → infrastructure/ hierarchy
- Configuration: `config/` module in demiarch-core

**Directory Structure:**
```
demiarch/
├── demiarch-core/          // Shared business logic
│   ├── domain/
│   ├── application/
│   ├── infrastructure/
│   ├── tests/
│   └── Cargo.toml
├── demiarch-cli/          // CLI with ratatui
│   ├── src/
│   ├── tests/
│   └── Cargo.toml
├── demiarch-gui/          // Tauri + React GUI
│   ├── src/
│   ├── src-tauri/
│   ├── src/               // React frontend
│   ├── tests/
│   └── package.json
├── Cargo.toml             // Workspace root
└── package.json           // Workspace root (if needed)
```

**Test Organization:**
- Unit tests: Co-located with modules (`mod.rs` → `tests.rs` or `tests/` subdirectory)
- Integration tests: `tests/` directory at crate root
- E2E tests: Separate `e2e-tests/` directory (optional, for GUI testing)

### Format Patterns

**API Response Format:**

**Rules:**
- Use consistent wrapper: `{data: T, error: Error | null}`
- Errors include: `code` (string), `message` (string), `details` (optional)
- Success: `{data: T, error: null}`

**Examples:**
```typescript
// Success
{ data: { projectId: "123" }, error: null }

// Error
{
  data: null, 
  error: { 
    code: "PROJECT_NOT_FOUND", 
    message: "Project with ID 123 not found", 
    details: { projectId: "123" }
  }
}
```

**Data Format:**

**Rules:**
- JSON: camelCase for top-level keys, snake_case for database storage
- Booleans: `true/false` in JSON, convert to Rust `bool`
- Dates: ISO 8601 strings in JSON, parse to Rust `chrono::DateTime`

**Examples:**
```json
{
  "projectId": "123",
  "createdAt": "2026-01-19T12:00:00Z",
  "isActive": true
}
```

### Communication Patterns

**Event System Patterns:**

**Rules:**
- Event names: snake_case, past tense (e.g., `project_created`)
- Event payload: Structured TypeScript interface
- Event versioning: `{EventName}V1`, `{EventName}V2`

**Examples:**
```typescript
// Event definitions
interface ProjectCreatedEvent {
  projectId: string;
  projectName: string;
  timestamp: string;
}

interface SessionStartedEvent {
  sessionId: string;
  projectId: string;
  userId: string | null;
}
```

**State Management Patterns:**

**Rules:**
- Zustand stores: Separate stores per domain (projects, agents, costs, ui)
- Actions: camelCase, prefix with domain (e.g., `setActiveProject`, `clearAgents`)
- Selectors: camelCase functions starting with `select` prefix (e.g., `selectActiveProject`)

**Examples:**
```typescript
// Zustand store structure
interface ProjectsStore {
  projects: Project[];
  activeProjectId: string | null;
  isLoading: boolean;
  setActiveProject: (id: string) => void;
  addProject: (project: Project) => void;
}

const useProjectsStore = create<ProjectsStore>((set) => ({
  projects: [],
  activeProjectId: null,
  isLoading: false,
  setActiveProject: (id) => set((state) => ({ activeProjectId: id })),
  addProject: (project) => set((state) => ({ projects: [...state.projects, project] })),
}));
```

### Process Patterns

**Error Handling Patterns:**

**Rules:**
- Domain errors: Enum per domain (e.g., `ProjectError`, `AgentError`)
- Application errors: `AppError` enum wrapping all domain errors
- Use `Result<T, E>` throughout Rust code
- Provide user-friendly messages for all errors

**Examples:**
```rust
// Domain errors
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    NotFound { id: String },
    InvalidName { name: String },
    DatabaseError(#[from] sqlx::Error),
}

// Application error
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    Project(#[from] ProjectError),
    Agent(#[from] AgentError),
    Config(String),
}
```

**Loading State Patterns:**

**Rules:**
- Global loading state: `isLoading: boolean` in UI store
- Per-operation loading: `isGenerating`, `isSaving`, `isSyncing`
- Loading UI: Show spinner/progress bar during loading states

**Examples:**
```typescript
// Zustand store with loading states
interface UIStore {
  isLoading: boolean;
  isGenerating: boolean;
  isSaving: boolean;
  setLoading: (loading: boolean) => void;
}

const useUIStore = create<UIStore>((set) => ({
  isLoading: false,
  isGenerating: false,
  isSaving: false,
  setLoading: (loading) => set({ isLoading: loading })),
}));
```

### Enforcement Guidelines

**All AI Agents MUST:**

1. **Follow Naming Conventions** - All tables, columns, APIs, components, functions must follow specified naming patterns
2. **Match DDD Structure** - All code must align with domain/ → application/ → infrastructure/ hierarchy
3. **Use Result Types** - All Rust functions must return `Result<T, E>` or `Option<T>` where appropriate
4. **Implement Traits** - All repository traits and command traits must be defined in `interfaces/` and implemented in `infrastructure/`
5. **SOLID Principles** - All code must follow Single Responsibility, Open/Closed, Liskov, Interface Segregation, Dependency Inversion
6. **Type Safety** - All Tauri commands must use auto-generated TypeScript types
7. **State Consistency** - All state updates must go through Zustand actions, never direct mutation
8. **Error Handling** - All errors must provide user-friendly messages with actionable context

**Pattern Enforcement:**

- **Verify in code reviews** - Check naming, structure, and pattern adherence
- **Lint rules**: Add clippy rules for Rust, ESLint rules for TypeScript
- **Pre-commit hooks**: Enforce patterns before commits
- **CI/CD checks**: Automated pattern verification in CI pipeline

**Pattern Examples:**

**Good Examples:**
```rust
// Database naming
CREATE TABLE projects (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

// API naming
#[tauri::command]
async fn create_project(name: String) -> Result<Project, String>

// DDD structure
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: &Project) -> Result<Project>;
}

// SOLID - Dependency Inversion
pub struct ProjectService<R: ProjectRepository> {
    repository: R,
}
```

**Anti-Patterns:**

❌ Inconsistent casing (mixing camelCase, snake_case, PascalCase)
❌ Global mutable state (use state management library)
❌ Raw SQL strings (use sqlx compile-time queries or parameterized queries)
❌ Direct database mutations without Result types
❌ Error propagation without context (provide meaningful error messages)
❌ Duplicate logic across modules (extract to shared utilities)
❌ Domain logic in UI components (move to application services)

## Project Structure & Boundaries

### Complete Project Directory Structure

```
demiarch/
├── README.md
├── LICENSE
├── .gitignore
├── Cargo.toml                      # Workspace root configuration
├── package.json                    # Workspace root (if needed)
├── demiarch-core/                  # Core business logic (shared library)
│   ├── Cargo.toml                 # Core dependencies
│   ├── src/
│   │   ├── domain/               # Domain entities and business rules
│   │   ├── projects/
│   │   ├── agents/
│   │   ├── context/
│   │   ├── skills/
│   │   ├── code_generation/
│   │   ├── plugins/
│   │   ├── cost/
│   │   ├── security/
│   │   └── recovery/
│   ├── application/          # Use cases/orchestrators (DDD application services)
│   │   ├── generate_code.rs
│   │   ├── create_project.rs
│   │   ├── manage_session.rs
│   │   └── ...
│   ├── infrastructure/       # External integrations
│   │   ├── db/             # SQLite with sqlite-vec
│   │   ├── llm/            # OpenRouter client
│   │   ├── git/            # git2-rs integration
│   │   └── wasm/           # Wasmtime plugin runtime
│   ├── interfaces/            # Trait definitions
│   ├── config/               # Configuration management
│   └── tests/                   # Unit tests (co-located)
├── demiarch-cli/                   # CLI with ratatui
│   ├── Cargo.toml                 # CLI dependencies
│   ├── src/
│   ├── main.rs               # CLI entry point
│   ├── tui/                  # Terminal UI components
│   ├── commands/              # CLI command handlers
│   └── tests/                   # CLI tests
├── demiarch-gui/                   # Tauri GUI with React
│   ├── Cargo.toml                 # Tauri dependencies
│   ├── src/
│   ├── src-tauri/            # Rust Tauri backend
│   │   ├── main.rs           # Tauri entry point
│   │   ├── commands/         # Tauri command implementations
│   │   ├── lib.rs             # Core library using demiarch-core
│   │   └── plugin/           # Tauri plugin configurations
│   ├── src/                  # React frontend
│   │   ├── main.tsx          # React entry point
│   │   ├── components/        # shadcn/ui components
│   │   │   ├── ui/          # Base UI components
│   │   │   ├── features/      # Feature-specific components
│   │   │   │   ├── projects/
│   │   │   │   ├── agents/
│   │   │   │   ├── costs/
│   │   │   │   └── settings/
│   │   ├── lib/              # Shared utilities (Zustand stores)
│   │   ├── hooks/            # React hooks
│   │   ├── types/            # TypeScript types
│   │   ├── styles/           # Tailwind CSS (global styles)
│   │   ├── assets/                # Static assets
│   │   └── tests/               # React tests
│   ├── package.json                # React dependencies
│   ├── tailwind.config.js           # Tailwind configuration
│   ├── tsconfig.json              # TypeScript configuration
│   ├── vite.config.ts              # Vite configuration
│   ├── Tauri.toml               # Tauri configuration
│   ├── .env.example               # Environment variables template
├── scripts/                      # Build/deployment scripts
├── docs/                         # Additional documentation
├── .github/                       # GitHub workflows
│   └── workflows/
│       └── ci.yml                 # Continuous integration
└── .env.example                   # Environment variables template
```

### Architectural Boundaries

#### API Boundaries

**External API Integrations:**
- OpenRouter API (LLM access) - Implemented in `demiarch-core/infrastructure/llm/`
- No external REST endpoints currently (future API to be added)

**Internal Service Boundaries:**
- Application services orchestrate domain logic
- Infrastructure layer handles external dependencies
- Clear separation: domain → application → infrastructure

#### Component Boundaries

**Frontend Component Communication:**
- Parent-child data flow via React props
- Cross-component state via Zustand stores
- Event-driven communication between unrelated components

**State Management Boundaries:**
- Zustand stores organize state by domain
- No direct state mutations (use only store actions)
- State sync with Tauri commands via invoke pattern

#### Service Communication Patterns:**
- Services communicate via Rust traits (interfaces/)
- Tauri commands expose application services to frontend
- Event-driven integration for cross-cutting concerns (hooks, plugins)

#### Data Boundaries

**Database Schema Boundaries:**
- SQLite database per project (separate files)
- Schema migrations handled within each project database
- Vector embeddings stored in sqlite-vec tables

**Data Access Patterns:**
- Repository traits define data access interfaces
- Concrete implementations in infrastructure/db/
- All data access goes through repository abstraction

#### External Data Integration Points:**
- JSONL export/import for git sync
- Plugin configuration files (license, capabilities)
- Checkpoint files for recovery system

### Integration Points

#### Internal Communication

**Component Communication:**
- React components communicate via:
  - Props for parent-child data flow
  - Zustand stores for cross-component state
  - Context API for deeply nested state
  - Tauri commands called from React components
  - Event bus for lifecycle hooks

**Service Communication:**
- Application services communicate via:
  - Trait-based interfaces in `interfaces/`
  - Dependency injection in constructors
  - Result types for error propagation
  - No circular dependencies

#### External Integrations

**OpenRouter API Integration:**
- LLM client in `demiarch-core/infrastructure/llm/`
- Retry logic with exponential backoff
- Model routing (RL-optimized selection)
- Rate limiting (Governor crate)

**Git Integration:**
- git2-rs wrapper in `demiarch-core/infrastructure/git/`
- Manual sync operations (no automatic commits)
- JSONL export/import for git compatibility
- File checksum tracking for conflict detection

**WASM Plugin System:**
- Wasmtime runtime in `demiarch-core/infrastructure/wasm/`
- Fuel-based CPU limiting with randomized limits
- Capability checks at host boundary
- License verification with ed25519 signatures
- Plugin loading from project-specific or global directories

#### Data Flow

**Database Flow:**
- Repository pattern with trait abstractions
- sqlx for async queries with compile-time checking
- Migrations for schema evolution
- Connection pooling with sqlx

**State Flow:**
- Zustand stores hold domain state
- React components subscribe to state via hooks
- State updates through store actions only
- Tauri commands update UI state via invoke pattern

**Cache Flow:**
- Async caching with moka/lru
- LLM prompt caching for token efficiency
- Embedding result caching for progressive disclosure

### File Organization Patterns

**Configuration Files:**
- `.env.example` - Environment variable template
- `Tauri.toml` - Tauri configuration (GUI)
- `vite.config.ts` - Vite bundling configuration
- `tailwind.config.js` - Tailwind CSS configuration
- `tsconfig.json` - TypeScript compiler options
- `Cargo.toml` - Workspace and crate dependencies

**Source Code Organization:**
- Domain-driven structure in `demiarch-core/src/`
- Feature-based components in `demiarch-gui/src/src/components/`
- Command handlers in `demiarch-cli/src/commands/`
- Shared traits in `demiarch-core/src/interfaces/`

**Test Organization:**
- Unit tests: Co-located with modules (`mod.rs` → `tests.rs` or `tests/` subdirectory)
- Integration tests: `tests/` at crate root
- E2E tests: Separate directories (GUI tests in `demiarch-gui/tests/e2e/`)

**Asset Organization:**
- Static assets: `demiarch-gui/public/`
- Icons and images: `demiarch-gui/public/assets/`
- CSS: `demiarch-gui/src/src/styles/` (Tailwind global styles)

### Development Workflow Integration

**Build Process Structure:**

**Development Server Structure:**
- Tauri dev server manages Rust + React hot reload
- Vite handles React HMR
- Watch mode monitors all crates for changes

**Build Process Structure:**

**Distribution Structure:**
- Tauri CLI builds application bundles
- Cross-platform installers: .app (macOS), .exe (Windows), .AppImage (Linux)
- Code signing configuration for each platform
- Bundle size optimization (Tree shaking, minification)

**Deployment Structure:**
- GitHub Actions CI pipeline for automated builds
- Security scanning (cargo-audit, npm audit) in CI
- Release tagging and changelog generation
- Platform-specific distribution via GitHub Releases

### Validation

This architecture document is complete and ready for validation. All major decisions have been made, implementation patterns defined, project structure mapped, and architectural boundaries documented.

**Next Steps:**
1. Proceed to Create Epics and Stories workflow with this architecture as input
2. Implement first story: Project initialization (Tauri setup)
3. Build out core domains incrementally following DDD structure
4. Add UI components using shadcn/ui + Tailwind
5. Implement CLI with ratatui sharing demiarch-core
