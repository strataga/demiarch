# Demiarch

> **🚧 Work in Progress** — This project is in early development. See [ROADMAP.md](ROADMAP.md) for current status and planned features.

An AI-powered app builder that generates code through conversational AI and intelligent orchestration. Works as both a web app and desktop application (via Tauri).

## Philosophy

- **Code Generation Tool, Not App Runner** - Demiarch generates code; you run your own projects
- **Intelligent Orchestration** - Auto Build processes features automatically with AI code generation
- **You Own Everything** - Edit freely, AI respects your changes during regeneration
- **Works Anywhere** - Web mode uses localStorage, desktop mode adds file system access

## GUI Features (Current)

The GUI is fully functional and provides:

| Feature | Description |
|---------|-------------|
| **Dashboard** | Overview of projects, costs tracking, and system health |
| **Projects** | Create projects with AI-assisted PRD generation via chat |
| **Kanban Board** | Drag-and-drop feature management across To Do, In Progress, Complete, Blocked |
| **Auto Build** | Toggle to automatically process features through AI code generation |
| **Agent Tracking** | Real-time view of agents with dependency and setup tracking |
| **Feature Details** | Comprehensive modal with build summary, git info, tabs for code/setup |
| **AI Feature Extraction** | Extract features from PRD automatically using AI |
| **Settings** | Configure OpenRouter API key and model selection |

### Auto Build Orchestrator

When enabled, Auto Build automatically:
1. Picks highest-priority pending features (P0 first)
2. Generates implementation code via AI
3. Tracks dependencies (npm packages, etc.) and setup requirements
4. Moves features through the pipeline
5. Logs all activity in the Build Log panel

### Feature Detail Modal

- **Build Summary** - Shows files generated, dependencies, setup steps
- **Git Branch Info** - Current branch and last updated timestamp
- **Tabbed Interface** - Overview, Code (with Open Folder/History), Setup
- **Retry Build** - One-click retry for blocked features

## Planned Features

| Feature | Description |
|---------|-------------|
| Image Generation | Text-to-image, image-to-image, upscaling, and inpainting via OpenRouter |
| Russian Doll Agents | 3-level hierarchical agent delegation (Orchestrator → Planner → Coder/Reviewer/Tester) |
| Learned Skills | Auto-extract debugging knowledge as reusable, persistent skills |
| Knowledge Graph | GraphRAG-powered skill discovery with entity extraction |
| Dynamic Model Routing | RL-optimized selection of specialized vs generalist models per task |
| Lifecycle Hooks | Extensible event handlers for session, generation, and error events |
| Multi-Project | Work on 3-5 projects concurrently with cross-project context |
| Recovery System | Checkpoints and rollback for safe experimentation |

## Project Structure

```
demiarch/
├── gui/                   # React + Tauri GUI (primary interface)
│   ├── src/
│   │   ├── components/    # Reusable UI components
│   │   ├── pages/         # Page components (Dashboard, Kanban, etc.)
│   │   ├── hooks/         # Custom React hooks (useAutoBuild, etc.)
│   │   └── lib/           # Services (api, ai, shell, git)
│   └── src-tauri/         # Tauri backend (Rust)
├── crates/
│   ├── demiarch-core/     # Core library (commands, agents, storage)
│   ├── demiarch-cli/      # CLI binary
│   ├── demiarch-tui/      # TUI monitor (optional real-time dashboard)
│   └── demiarch-plugins/  # Plugin system (WASM sandboxing, licensing)
├── docs/                  # Documentation
└── scripts/               # Utility scripts
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

### GUI (Recommended)

```bash
# Clone the repository
git clone https://github.com/strataga/demiarch.git
cd demiarch/gui

# Install dependencies
npm install

# Run in web browser (no Tauri needed)
npm run dev

# Open http://localhost:5173
```

### Desktop App (Tauri)

```bash
# Requires Rust and Tauri prerequisites
# See: https://tauri.app/v1/guides/getting-started/prerequisites

cd gui
npm install
npm run tauri dev
```

### CLI (Optional)

```bash
# From source
cargo install --path crates/demiarch-cli

# Or build all
cargo build --release
```

## Quick Start (GUI)

1. **Open the app** - Run `npm run dev` in the `gui` directory and open http://localhost:5173

2. **Configure API Key** - Go to Settings and enter your OpenRouter API key
   - Get one free at [openrouter.ai/keys](https://openrouter.ai/keys)

3. **Create a Project** - Click "New Project" on the Dashboard
   - Enter project name and framework
   - Optionally paste a PRD or chat with AI to create one

4. **Extract Features** - On the Project page, click "Extract Features"
   - AI will parse your PRD and create feature cards

5. **Open Kanban** - Navigate to the Kanban board for your project

6. **Enable Auto Build** - Toggle "Auto Build" in the header
   - Watch as features are automatically processed
   - View progress in the Build Log panel

7. **Review Features** - Click any feature card to see:
   - Generated code files
   - Dependencies required
   - Setup steps needed

## Configuration

### GUI Settings

In the Settings page, configure:
- **OpenRouter API Key** - Required for AI features
- **Model Selection** - Choose from Claude, GPT-4, Gemini, Llama, etc.

### CLI Configuration

```bash
# Set your OpenRouter API key
demiarch config set openrouter_api_key sk-or-...

# Set daily budget
demiarch config set cost_daily_limit_usd 10.0
```

## CLI Commands (Optional)

```bash
demiarch new          # Create new project
demiarch chat         # Conversational discovery
demiarch features     # Manage features
demiarch generate     # Generate code
demiarch watch        # TUI monitor
demiarch costs        # View usage & costs
demiarch doctor       # Health check
```

## Tech Stack

### GUI
- **Framework**: React 18 + TypeScript
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **State Management**: Zustand
- **Drag & Drop**: @dnd-kit
- **Icons**: Lucide React
- **Desktop**: Tauri (optional)

### Backend (CLI/TUI)
- **Language**: Rust
- **Database**: SQLite (with vector extensions)
- **TUI**: ratatui
- **LLM**: OpenRouter API (Claude, GPT-4, Gemini, Llama, etc.)

## Development

### GUI Development

```bash
cd gui

# Install dependencies
npm install

# Run development server
npm run dev

# Build for production
npm run build

# Run with Tauri (desktop)
npm run tauri dev
```

### CLI/Core Development

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

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=strataga/demiarch&type=date&legend=top-left)](https://www.star-history.com/#strataga/demiarch&type=date&legend=top-left)
