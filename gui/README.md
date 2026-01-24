# Demiarch GUI

A React-based GUI for Demiarch - the AI-powered app builder.

## Features

### Dashboard
- Project overview with feature counts
- Cost tracking display
- System health status (API key, database)

### Projects
- Create new projects with name, framework, and optional PRD
- AI-assisted PRD generation through conversational chat
- View and edit project details
- Delete projects

### Kanban Board
- Drag-and-drop feature management
- Four columns: To Do, In Progress, Complete, Blocked
- **Auto Build Toggle** - Automatically process features with AI
- **Build Log Panel** - Real-time activity log
- Search and filter features
- Priority badges (P0-P4)
- Due date tracking with overdue warnings

### Auto Build Orchestrator
When enabled, automatically:
1. Picks highest-priority pending features (P0 first)
2. Moves feature to "In Progress"
3. Generates implementation code via AI
4. Extracts dependencies (npm packages, etc.)
5. Identifies setup requirements
6. Stores generated code on feature
7. Moves feature to "Complete" (or "Blocked" on error)
8. Continues to next feature

### Feature Detail Modal
- **Build Summary Card** - Shows files, dependencies, setup counts
- **Git Branch Info** - Current branch and last updated
- **Tabbed Interface**:
  - Overview: Description, priority, due date, tags
  - Code: Generated files with Open Folder, View History, Copy Path
  - Setup: Dependencies and setup requirements
- **Retry Build** - One-click retry for blocked features
- Edit mode for updating feature details
- Delete feature with confirmation

### Agents Page
- View agent hierarchy during builds
- Expand to see:
  - Generated files
  - Dependencies
  - Setup requirements
- Clear completed agents

### Settings
- OpenRouter API key configuration
- Model selection (Claude, GPT-4, Gemini, Llama, etc.)
- Cost limit settings

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool
- **Tailwind CSS** - Styling
- **Zustand** - State management
- **@dnd-kit** - Drag and drop
- **Lucide React** - Icons
- **React Router** - Navigation
- **Tauri** - Desktop app (optional)

## Development

```bash
# Install dependencies
npm install

# Run development server (web)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview

# Run with Tauri (desktop)
npm run tauri dev
```

## Project Structure

```
src/
├── components/          # Reusable UI components
│   ├── AutoBuildModal.tsx
│   ├── BuildLogPanel.tsx
│   ├── ExtractFeaturesModal.tsx
│   ├── FeatureCreateModal.tsx
│   ├── FeatureDetailModal.tsx
│   ├── Layout.tsx
│   ├── SearchInput.tsx
│   ├── Skeleton.tsx
│   └── Toggle.tsx
├── hooks/               # Custom React hooks
│   ├── useAutoBuild.ts      # Auto Build orchestrator
│   └── useKeyboardShortcuts.ts
├── lib/                 # Services and utilities
│   ├── ai.ts            # OpenRouter AI integration
│   ├── api.ts           # API wrapper (Tauri/localStorage)
│   ├── beads.ts         # Beads issue tracking
│   ├── fileWriter.ts    # File system operations
│   ├── git.ts           # Git operations
│   └── shell.ts         # Shell utilities
├── pages/               # Page components
│   ├── Agents.tsx
│   ├── ConflictResolution.tsx
│   ├── Dashboard.tsx
│   ├── Kanban.tsx
│   ├── ProjectDetail.tsx
│   ├── Projects.tsx
│   └── Settings.tsx
├── App.tsx              # Root component with routing
├── main.tsx             # Entry point
└── index.css            # Global styles
```

## API Modes

The GUI works in two modes:

### Web Mode (localStorage)
- No backend required
- Data persists in browser localStorage
- Limited file system access (shows paths only)
- Great for trying out the app

### Desktop Mode (Tauri)
- Full file system access
- Git integration
- Write generated files to disk
- Requires Rust and Tauri prerequisites

## Configuration

### OpenRouter API Key
Required for AI features. Get one at [openrouter.ai/keys](https://openrouter.ai/keys).

Configure in Settings page or set in localStorage:
```javascript
localStorage.setItem('openrouter_api_key', 'sk-or-...');
```

### Available Models
- Claude Sonnet 4 (Recommended)
- Claude 3.5 Sonnet
- Claude 3 Opus
- GPT-4o
- GPT-4 Turbo
- Gemini Pro 1.5
- Llama 3.1 405B

## License

AGPL-3.0 - See [LICENSE](../LICENSE)
