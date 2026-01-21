# Demiarch: Product Requirements Document

## Executive Summary

Demiarch is a local-first, open-source AI app builder that generates code into users' local repositories through conversational AI. Users describe features in natural language, and hierarchical AI agents autonomously generate production-ready code for any major framework.

**Vision**: Enable anyone to build software through conversation, with complete code ownership and control.

**Target Users**:
- **Technical developers** seeking productivity gains through AI-assisted code generation
- **Non-technical users** who want to build applications without traditional programming

---

## 1. Core Philosophy

### Code Generation Tool, Not App Runner
- Demiarch generates code; users run their own projects
- All data stays local (SQLite database per project)
- No accounts required, no telemetry by default
- User owns everything and can edit freely
- AI respects user edits during regeneration

### Explicit Over Implicit
- No automatic git operations
- No background processes
- User controls when to sync, commit, push
- Every operation is a deliberate user action

---

## 2. Functional Requirements

| ID | Requirement | Description |
|----|-------------|-------------|
| FR1 | AI-Powered Code Generation | Conversational discovery interface that generates code into user's local repositories, supporting all major web, mobile, desktop, and backend frameworks |
| FR2 | Document Auto-Generation | Automated generation of PRD, Architecture, and UX design documents through AI collaboration |
| FR3 | Phase Planning | Breakdown of projects into phases with user stories, acceptance criteria, and implementation tracking |
| FR4 | Multi-Project Concurrency | Support for working on 3-5 projects simultaneously with cross-project context sharing and resource locking |
| FR5 | Russian Doll Agent System | 3-level hierarchical agent architecture (Orchestrator → Planner → Coder/Reviewer/Tester) with autonomous delegation |
| FR6 | Progressive Disclosure | Token-efficient context retrieval with 3-layer summaries (Index/Timeline/Full) achieving ~10x token savings |
| FR7 | Learned Skills Extraction | Autonomous extraction of debugging knowledge as reusable skills with embedding-based semantic search and RL quality optimization |
| FR8 | Dynamic Model Routing | RL-optimized selection between specialized (Codestral, Qwen-Math) and generalist (Claude, GPT-4o) models based on task type and learned performance |
| FR9 | Conflict Resolution | Smart detection and handling of user edits vs AI-regenerated code with manual merge support |
| FR10 | Plugin System | Extensible framework for code generation with WASM sandboxing, offline license verification (ed25519), and pricing models (free, paid, usage, trial, watermark) |
| FR11 | Storage Pattern | SQLite primary storage with JSONL git-sync format for explicit, non-automatic sync operations |
| FR12 | Recovery System | Checkpoint-based rollback for safe experimentation with generation transactions and cleanup tracking |
| FR13 | Lifecycle Hooks | Extensible event handling for session_start, session_end, pre_generation, post_generation, on_error, on_checkpoint |
| FR14 | Multi-Interface Support | CLI (TUI with ratatui), GUI (Tauri + React), and future REST API using shared command library pattern |
| FR15 | Cost Management | Real-time cost tracking with budget enforcement, daily limits, and cost alerts |
| FR16 | Offline Support | Degraded mode with operation queuing and automatic retry when connectivity restored |
| FR17 | Security Infrastructure | AES-GCM encryption for API keys with argon2 key derivation, audit logging, update signature verification, semantic filtering for prompt injection prevention |
| FR18 | Conversational AI Interface | Natural language chat interface for defining, refining, and generating application features |
| FR19 | Kanban Board for Project Management | Visual drag-and-drop interface where users manage project phases, track feature progress, and organize development workflow |
| FR20 | Agent Visualization | Real-time visual representation of Russian Doll agent hierarchy showing agent spawning, task decomposition, and delegation flow |

---

## 3. Non-Functional Requirements

### Security
- API keys encrypted at rest using AES-GCM with machine-based key derivation (argon2)
- OS keyring storage preferred, encrypted SQLite fallback with zeroize
- Plugin sandboxing using WASM with CPU/memory limits and multi-tier verification
- Offline license verification with ed25519 cryptographic signatures
- Comprehensive audit logging for security events
- Rate limiting (requests/minute, tokens/minute)
- Semantic filter at embedding retrieval to prevent indirect prompt injection
- Checkpoint file signing with ed25519 for integrity verification
- Plugin signing ecosystem with three-tier trust model (built-in, verified, unverified)

### Performance
- Card creation under 2 seconds after chat confirmation
- Drag-and-drop at 60fps with no lag
- Chat response under 5 seconds for simple confirmations, under 15 for complex queries
- Progressive disclosure reduces token usage by ~10x through layered context summaries
- Vector-based semantic search for skills and context (1536-dimensional embeddings)
- Efficient conflict detection using file checksums

### Reliability
- Network resilience with exponential backoff retry (max 3 retries, 30s max delay)
- Automatic model fallback (default → Haiku → GPT-4o)
- Database migrations for schema evolution
- Recovery system with checkpoint-based rollback
- Generation transaction tracking for cleanup on failure

### Usability
- Multiple interfaces (CLI, GUI, API) sharing same command library
- Explicit control (no automatic git operations, no background processes)
- Clear error messages with actionable suggestions
- Cost dashboard with per-model and per-feature breakdown

---

## 4. Technology Stack

### Backend (Rust)
- **Async Runtime**: Tokio 1.41
- **Database**: sqlx 0.8, rusqlite 0.32 with sqlite-vec (v0.1.6) for embeddings
- **GUI Framework**: Tauri 2.1
- **CLI**: clap 4.5, ratatui 0.29
- **WASM Runtime**: wasmtime 27.0
- **Cryptography**: ed25519-dalek 2.1, aes-gcm 0.10, argon2 0.5, zeroize 1.8
- **Git Integration**: git2 0.19
- **Rate Limiting**: governor 0.7

### Frontend (React + TypeScript)
- **UI Framework**: React 18.3, TypeScript 5.6
- **State Management**: Zustand 5.0
- **Styling**: Tailwind CSS 3.4, shadcn/ui (Radix UI components)
- **Editor**: Monaco Editor 0.52
- **Icons**: Lucide React 0.460

### Architecture Pattern
- Cargo Workspace with 3 crates: demiarch-core, demiarch-cli, demiarch-gui
- Domain-Driven Design (DDD) with SOLID principles
- CLI as Library pattern for shared command implementations

---

## 5. Epic Breakdown

### Epic 1: Foundation & Project Setup (6 stories)
Users can install Demiarch, create projects, and configure core infrastructure (storage, security, settings).

**Stories**:
1. Setup CI/CD Pipeline with Security Scanning
2. Initialize Project with Starter Template
3. Set Up SQLite Database per Project
4. Configure API Key Storage with Encryption
5. Implement Basic CLI Interface
6. Track Basic LLM Costs

### Epic 2: Conversational Development (5 stories)
Users can describe features through natural language chat and have AI generate code, PRD/architecture documents, and break down features into phases.

**Stories**:
1. Implement Chat Interface with Conversation Threading
2. Integrate OpenRouter LLM Client
3. Generate Code from Natural Language Description
4. Auto-Generate PRD and Architecture Documents
5. Break Down Features into Phases with User Stories

### Epic 3: Project Management & Kanban (5 stories)
Users can visualize their development progress through a drag-and-drop kanban board and track features through complete lifecycle.

**Stories**:
1. Create Kanban Board Layout with Columns
2. Implement Feature Cards with Drag-and-Drop
3. Display Feature Details on Card Expansion
4. Basic Project Switching UI
5. Basic Agent Status Visualization

### Epic 4: Code Safety & Recovery (5 stories)
Users can generate code safely with automatic checkpoints, rollback capability, and intelligent conflict resolution when AI regenerates code they've edited.

**Stories**:
1. Create Automatic Checkpoints Before Major Changes
2. Implement Rollback to Previous Checkpoint
3. Detect User Edits to Generated Code
4. Display Conflict Resolution Interface
5. Export to JSONL for Git Sync

### Epic 5: Advanced Agent Capabilities (6 stories)
Users can leverage Russian Doll agent hierarchy for sophisticated code generation with progressive disclosure context management and learned skills for faster debugging.

**Stories**:
1. Implement Russian Doll Agent Hierarchy
2. Implement Progressive Disclosure for Context Management
3. Extract and Store Learned Skills
4. Implement Semantic Search for Skills Retrieval
5. Implement Dynamic Model Routing with RL
6. Full Agent Visualization with Hierarchy Tree

### Epic 6: Multi-Project Workspace (5 stories)
Users can work on 3-5 projects simultaneously with cross-project context sharing, unified sessions, and visual project switching.

**Stories**:
1. Implement Global Session Management
2. Implement Resource Locking for Concurrency
3. Cross-Project Search with Opt-In
4. Session Recovery on Restart
5. Session End and Cleanup

### Epic 7: Plugin Ecosystem (7 stories)
Users can install, verify, and use third-party plugins for additional frameworks and capabilities with WASM sandboxing and offline license verification.

**Stories**:
1. Install Built-in Framework Plugins
2. Load and Execute WASM Plugins
3. Offline License Verification with ed25519
4. Implement Three-Tier Plugin Trust Model
5. Configure Plugin Capabilities and Permissions
6. Implement Plugin Pricing Models
7. Lifecycle Hooks for Plugin Events

### Epic 8: Offline & Operations (6 stories)
Users can work without connectivity with queued operations, automatic retry, and graceful degradation when offline.

**Stories**:
1. Detect Network Connectivity and Enter Offline Mode
2. Queue Operations During Offline Mode
3. Automatic Retry on Connectivity Restore
4. Graceful Degradation for Offline Features
5. Cost Alerts with Daily Limits
6. Project Rate Limits and Resource Throttling

---

## 6. User Experience Design

### Design System
- **Color Palette**: Neural network-inspired dark theme
  - Background: #0d1b2a (deep), #1b263b (mid), #253346 (surface)
  - Accents: #00f5d4 (teal), #f72585 (magenta), #ffc300 (amber)
- **Typography**: IBM Plex Sans (UI), Fira Code (code/technical content)
- **Visual Language**: Neural network visualization with concentric rings, animated nodes, flowing connections, glassmorphism panels

### Agent Color Mapping
- **Teal**: Orchestrator (Level 1)
- **Magenta**: Planner (Level 2)
- **Amber**: Workers - Coder/Reviewer/Tester (Level 3)

### Core Experiences
1. **Talk to Build**: Conversational AI interface for natural language feature definition
2. **Drag to Track**: Kanban board for visual project management

### Responsiveness
- Desktop: 600px neural visualization
- Tablet: 400px scaled
- Mobile: 100% stacked layout

---

## 7. Database Schema (Key Tables)

### Core Tables
- `projects` - Project metadata and configuration
- `user_preferences` - User settings (theme, limits, defaults)
- `phases` - Project phases (Discovery, Planning, Building, Complete)
- `features` - Individual features with acceptance criteria
- `chat_messages` - Conversation history with LLM
- `agent_executions` - Agent hierarchy and execution tracking

### Code Generation Tables
- `generated_code` - AI-generated files with checksums
- `user_code_edits` - Tracked user modifications
- `checkpoints` - State snapshots for rollback
- `conflict_resolutions` - Conflict resolution decisions

### Plugin Tables
- `installed_plugins` - Plugin metadata and status
- `plugin_licenses` - License verification records
- `plugin_usage` - Usage-based pricing tracking

### Cost & Operations Tables
- `llm_usage` - Token and cost tracking per model
- `cost_alerts` - Budget threshold notifications
- `global_sessions` - Multi-project session state
- `resource_locks` - Concurrency management

---

## 8. Security Model

### API Key Protection
- OS keyring storage preferred (keyring crate)
- Encrypted SQLite fallback with AES-GCM
- Argon2 key derivation from machine ID
- ChaChaRng for cryptographically secure nonce generation
- Zeroize for immediate memory wiping

### Plugin Security
- WASM sandboxing with wasmtime
- Fuel-based CPU limiting (10M fuel max, randomized)
- Three-tier trust model: Built-in, Verified, Unverified
- Ed25519 signature verification for verified plugins
- Capability-based permission system

### Data Integrity
- Ed25519 checkpoint signing
- SHA-256 file checksums for conflict detection
- Semantic filtering for prompt injection prevention

---

## 9. Success Metrics

| Metric | Target |
|--------|--------|
| Feature generation success rate | >90% |
| User code preservation during regeneration | 100% |
| Checkpoint restore success rate | 100% |
| Token efficiency (progressive disclosure) | 10x savings |
| Card creation latency | <2 seconds |
| Drag-and-drop frame rate | 60 fps |

---

## 10. Implementation Status

**Total: 8 Epics, 45 Stories**

| Epic | Status | Stories Ready |
|------|--------|---------------|
| 1. Foundation & Project Setup | In Progress | 6/6 |
| 2. Conversational Development | Ready | 5/5 |
| 3. Project Management & Kanban | Ready | 5/5 |
| 4. Code Safety & Recovery | Ready | 5/5 |
| 5. Advanced Agent Capabilities | Ready | 6/6 |
| 6. Multi-Project Workspace | Ready | 5/5 |
| 7. Plugin Ecosystem | Ready | 7/7 |
| 8. Offline & Operations | Ready | 6/6 |

**Current Sprint Focus**: Epic 1 (Foundation) - Stories 1-1 (in review), 1-2 (in progress)