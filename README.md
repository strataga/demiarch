# Demiarch

> **🚧 Work in Progress** — This project is in early development. See [ROADMAP.md](ROADMAP.md) for current status and planned features.

A local-first, open-source AI app builder that generates code into your local repositories through conversational AI.

## Philosophy

- **Code Generation Tool, Not App Runner** - Demiarch generates code; you run your own projects
- **Local-First** - All data stays local (SQLite), no accounts required, no telemetry
- **Explicit Over Implicit** - No automatic git operations, no background processes
- **You Own Everything** - Edit freely, AI respects your changes during regeneration

## Features

| Feature | Description |
|---------|-------------|
| Conversational Discovery | Chat with AI to define project requirements |
| Document Generation | Auto-generate PRD, Architecture, UX docs |
| Image Generation | Text-to-image, image-to-image, upscaling, and inpainting via OpenRouter |
| Phase Planning | Break projects into phases with user stories |
| Multi-Framework | Next.js, React, Vue, Flutter, iOS, Android, FastAPI, Go, and more |
| Russian Doll Agents | 3-level hierarchical agent delegation (Orchestrator → Planner → Coder/Reviewer/Tester) |
| Learned Skills | Auto-extract debugging knowledge as reusable, persistent skills |
| Knowledge Graph | GraphRAG-powered skill discovery with entity extraction and multi-hop traversal |
| Progressive Disclosure | Token-efficient context retrieval (~10x savings via layered summaries) |
| Dynamic Model Routing | RL-optimized selection of specialized vs generalist models per task |
| Lifecycle Hooks | Extensible event handlers for session, generation, and error events |
| Multi-Project | Work on 3-5 projects concurrently with cross-project context |
| Recovery System | Checkpoints and rollback for safe experimentation |
| Cost Management | Per-model pricing, daily budgets, alerts, usage tracking |

## Project Structure

```
demiarch/
├── crates/
│   ├── demiarch-core/     # Core library (commands, agents, storage)
│   ├── demiarch-cli/      # CLI binary
│   ├── demiarch-tui/      # TUI monitor (optional real-time dashboard)
│   └── demiarch-plugins/  # Plugin system (WASM sandboxing, licensing)
├── gui/                   # Tauri app (future)
├── plugins/               # Bundled/example plugins
├── docs/                  # Documentation & PRD
└── migrations/            # Database migrations
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  User Interfaces                     │
│   ┌─────────┐   ┌─────────┐   ┌─────────┐          │
│   │   CLI   │   │   TUI   │   │   GUI   │          │
│   └────┬────┘   └────┬────┘   └────┬────┘          │
└────────┼─────────────┼─────────────┼────────────────┘
         │             │             │
         └─────────────┴─────────────┘
                       │
         ┌─────────────▼─────────────┐
         │     demiarch-core         │
         │  (commands, agents, db)   │
         └───────────────────────────┘
```

The CLI, TUI, and GUI all use `demiarch-core` as a library. The TUI provides a real-time dashboard showing agent activity, costs, and progress across all projects.

### Agent Hierarchy (Russian Doll Pattern)

```
User Request
     │
     ▼
┌─────────────────┐
│  Orchestrator   │  Level 1: Top-level coordinator
└────────┬────────┘
         │ AgentTool call
         ▼
┌─────────────────┐
│    Planner      │  Level 2: Decomposes features into tasks
└────────┬────────┘
         │ AgentTool calls
    ┌────┴────┬────────┐
    ▼         ▼        ▼
┌───────┐ ┌────────┐ ┌───────┐
│ Coder │ │Reviewer│ │Tester │  Level 3: Execute specific tasks
└───────┘ └────────┘ └───────┘
```

See [docs/russian-doll.jpg](docs/russian-doll.jpg) and [docs/multi-turn-reasoning.jpg](docs/multi-turn-reasoning.jpg) for architectural diagrams.

## Installation

```bash
# From source
cargo install --path crates/demiarch-cli

# Or build all
cargo build --release
```

## Quick Start

```bash
# Create a new Next.js project
demiarch new viral-shorts --framework nextjs --repo https://github.com/user/viral-shorts
cd viral-shorts

# Install Tailwind CSS and setup configuration
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p

# Install shadcn/ui components
npx shadcn-ui@latest init

# Start conversational discovery
demiarch chat

# Generate code for a feature
demiarch generate feat-abc123

# Monitor in real-time (TUI)
demiarch watch
```

## Configuration

```bash
# Set your OpenRouter API key
demiarch config set openrouter_api_key sk-or-...

# Set daily budget
demiarch config set cost_daily_limit_usd 10.0

# Set model routing preference
demiarch routing set-preference balanced
```

## CLI Commands

```bash
demiarch new          # Create new project
demiarch chat         # Conversational discovery
demiarch features     # Manage features
demiarch generate     # Generate code
demiarch watch        # TUI monitor
demiarch skills       # Manage learned skills
demiarch routing      # Model routing config
demiarch hooks        # Lifecycle hooks management
demiarch context      # Context memory management
demiarch image        # Image generation and manipulation
demiarch graph        # Explore knowledge graph (entities, relationships)
demiarch costs        # View usage & costs
demiarch sync         # Export/import for git
demiarch doctor       # Health check
```

### Image Commands

```bash
# Generate an image from text
demiarch image generate "A futuristic city at sunset" --output city.png

# Transform an existing image
demiarch image transform input.png "Make it look like a watercolor painting"

# Upscale an image (2x or 4x)
demiarch image upscale input.png --scale 2

# Inpaint (edit) a region using a mask
demiarch image inpaint photo.png mask.png "Replace with a sunset"

# List available models
demiarch image models
```

### Knowledge Graph Commands

```bash
# Show graph statistics
demiarch graph stats

# Search for entities
demiarch graph search "async runtime"

# Explore an entity's neighborhood
demiarch graph explore "tokio" --depth 2

# List entities by type
demiarch graph list --type library

# Show detailed view with relationships
demiarch graph explore "error handling" --tree
```

## Tech Stack

For viral-shorts project:
- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **UI Components**: shadcn/ui
- **State Management**: Zustand
- **Form Handling**: React Hook Form
- **Animation**: Framer Motion

Core Demiarch Stack:
- **Language**: Rust 
- **Database**: SQLite (with vector extensions for semantic search)
- **TUI**: ratatui
- **GUI**: Tauri + React + TypeScript
- **LLM**: OpenRouter API (Claude, GPT-4, etc.)
- **Image AI**: OpenRouter (Gemini, DALL-E 3, Stable Diffusion XL, Nano Banana Pro)

## Development

```bash
# Run CLI
cargo run -p demiarch-cli -- --help

# Run TUI
cargo run -p demiarch-tui

# Run tests
cargo test --workspace

# Check all
cargo clippy --workspace
```

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0).

This means:
- You can use, modify, and distribute this software
- If you modify it and provide it as a service (SaaS), you must release your source code
- All derivatives must also be AGPL-3.0

For commercial licensing options, please contact [TODO].

## Acknowledgments

Architecture inspired by:
- [Claudeception](https://github.com/blader/Claudeception) - Autonomous skill extraction
- [Claude-mem](https://github.com/thedotmack/claude-mem) - Progressive disclosure & lifecycle hooks
- [Ralph-TUI](https://github.com/subsy/ralph-tui) - Agent orchestration patterns
