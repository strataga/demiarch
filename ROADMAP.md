# Demiarch Roadmap

This document outlines the development roadmap for Demiarch.

## Architecture References

The architecture is informed by several key patterns:

- **Russian Doll Pattern** ([docs/russian-doll.jpg](docs/russian-doll.jpg)) - Hierarchical Task Decomposition with 3-level agent hierarchy
- **Multi-turn Reasoning** ([docs/multi-turn-reasoning.jpg](docs/multi-turn-reasoning.jpg)) - Dynamic model routing with specialized vs generalist LLMs
- **Claudeception** - Autonomous skill extraction with quality gating
- **Claude-mem** - Progressive disclosure for token-efficient context retrieval

---

## Phase 1: Foundation (Current)

**Goal:** Get the CLI functional with core features.

### Milestone 1.1: Project Skeleton ✅
- [x] Initialize Cargo workspace
- [x] Create crate structure (core, cli, tui, plugins)
- [x] Set up AGPL-3.0 licensing
- [x] Define module structure for all crates
- [x] Build passes (`cargo check --workspace`)
- [x] Tests pass (`cargo test --workspace`)
- [ ] Database schema implementation (from PRD)
- [ ] Basic configuration management

### Milestone 1.2: Core Commands
- [ ] `demiarch new` - Create projects
- [ ] `demiarch chat` - Conversational discovery
- [ ] `demiarch features` - Feature management
- [ ] `demiarch generate` - Code generation (basic)
- [ ] `demiarch sync` - SQLite <-> JSONL export

### Milestone 1.3: LLM Integration
- [ ] OpenRouter API client
- [ ] Model fallback chain
- [ ] Rate limiting
- [ ] Cost tracking

## Phase 2: Agent System

**Goal:** Implement the Russian Doll hierarchical agent system.

### Milestone 2.1: Base Agents
- [ ] Orchestrator agent
- [ ] Planner agent
- [ ] Coder agent
- [ ] Reviewer agent
- [ ] Tester agent

### Milestone 2.2: Context Management
- [ ] Basic context windowing
- [ ] Message summarization
- [ ] Progressive disclosure (3-layer retrieval)

### Milestone 2.3: Learned Skills
- [ ] Skill extraction from debugging sessions
- [ ] Skill matching (semantic + keyword)
- [ ] Quality gating and RL feedback

## Phase 3: Advanced Features

**Goal:** Dynamic model routing, hooks, and cost optimization.

### Milestone 3.1: Model Routing
- [ ] Task type classification
- [ ] Specialized model routing
- [ ] RL-based optimization
- [ ] Performance tracking

### Milestone 3.2: Lifecycle Hooks
- [ ] Hook registration system
- [ ] Built-in hooks (skill extraction, summarization)
- [ ] Script/plugin handlers

### Milestone 3.3: Cost Management
- [ ] Budget enforcement
- [ ] Daily limits
- [ ] Per-project tracking
- [ ] Alerts

## Phase 4: TUI & Plugins

**Goal:** Real-time monitoring and extensibility.

### Milestone 4.1: TUI Dashboard
- [ ] Multi-project view
- [ ] Agent activity stream
- [ ] Cost dashboard
- [ ] Skill activations

### Milestone 4.2: Plugin System
- [ ] WASM sandboxing (wasmtime)
- [ ] Plugin manifest validation
- [ ] Offline license verification
- [ ] Plugin marketplace API

## Phase 5: GUI (Tauri)

**Goal:** Desktop application with full feature parity.

### Milestone 5.1: Tauri Setup
- [ ] React + TypeScript frontend
- [ ] Tauri backend integration
- [ ] demiarch-core Rust bindings

### Milestone 5.2: Core UI
- [ ] Project management
- [ ] Chat interface
- [ ] Feature board
- [ ] Code diff viewer

### Milestone 5.3: Advanced UI
- [ ] Agent visualization
- [ ] Cost charts
- [ ] Settings panel

---

## Current Crate Structure

```
crates/
├── demiarch-core/          # Core library
│   └── src/
│       ├── commands/       # project, feature, generate, sync, chat
│       ├── agents/         # orchestrator, planner, coder, reviewer, tester
│       ├── storage/        # database, jsonl, migrations
│       ├── llm/            # OpenRouter client
│       ├── cost/           # Budget enforcement
│       ├── skills/         # Learned skills system
│       ├── context/        # Progressive disclosure
│       ├── routing/        # Dynamic model routing
│       ├── hooks/          # Lifecycle hooks
│       ├── config/         # Configuration
│       └── error.rs        # Error types with codes
│
├── demiarch-cli/           # CLI binary (`demiarch`)
├── demiarch-tui/           # TUI binary (`demiarch-tui`)
└── demiarch-plugins/       # WASM plugin system
    └── src/
        ├── loader.rs       # Plugin loading
        ├── sandbox.rs      # WASM sandboxing (wasmtime)
        ├── license.rs      # Offline license verification
        └── registry.rs     # Plugin marketplace
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to get involved.

## Tracking

- GitHub Issues: Feature requests, bugs
- GitHub Projects: Sprint planning
- This roadmap: High-level vision
