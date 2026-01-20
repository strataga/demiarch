---
stepsCompleted: [1, 2, 3, 4]
inputDocuments: ["docs/PRD.md", "_bmad-output/planning-artifacts/architecture.md", "_bmad-output/planning-artifacts/ux-design-specification.md", "docs/multi-turn-reasoning.jpg", "docs/russian-doll.jpg"]
workflowType: 'create-epics-and-stories'
project_name: 'demiarch'
user_name: 'Jason'
date: '2026-01-19'
---

# Demiarch - Epic Breakdown

## Overview

This document provides complete epic and story breakdown for demiarch, decomposing requirements from PRD, UX Design, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: AI-Powered Code Generation - Conversational discovery interface that generates code into user's local repositories, supporting all major web, mobile, desktop, and backend frameworks
FR2: Document Auto-Generation - Automated generation of PRD, Architecture, and UX design documents through AI collaboration
FR3: Phase Planning - Breakdown of projects into phases with user stories, acceptance criteria, and implementation tracking
FR4: Multi-Project Concurrency - Support for working on 3-5 projects simultaneously with cross-project context sharing and resource locking
FR5: Russian Doll Agent System - 3-level hierarchical agent architecture (Orchestrator → Planner → Coder/Reviewer/Tester) with autonomous delegation
FR6: Progressive Disclosure - Token-efficient context retrieval with 3-layer summaries (Index/Timeline/Full) achieving ~10x token savings
FR7: Learned Skills Extraction - Autonomous extraction of debugging knowledge as reusable skills with embedding-based semantic search and RL quality optimization
FR8: Dynamic Model Routing - RL-optimized selection between specialized (Codestral, Qwen-Math) and generalist (Claude, GPT-4o) models based on task type and learned performance
FR9: Conflict Resolution - Smart detection and handling of user edits vs AI-regenerated code with manual merge support
FR10: Plugin System - Extensible framework for code generation with WASM sandboxing, offline license verification (ed25519), and pricing models (free, paid, usage, trial, watermark)
FR11: Storage Pattern - SQLite primary storage with JSONL git-sync format for explicit, non-automatic sync operations
FR12: Recovery System - Checkpoint-based rollback for safe experimentation with generation transactions and cleanup tracking
FR13: Lifecycle Hooks - Extensible event handling for session_start, session_end, pre_generation, post_generation, on_error, on_checkpoint
FR14: Multi-Interface Support - CLI (TUI with ratatui), GUI (Tauri + React), and future REST API using shared command library pattern
FR15: Cost Management - Real-time cost tracking with budget enforcement, daily limits, and cost alerts
FR16: Offline Support - Degraded mode with operation queuing and automatic retry when connectivity restored
FR17: Security Infrastructure - AES-GCM encryption for API keys with argon2 key derivation, audit logging, update signature verification, semantic filtering for prompt injection prevention
FR18: Conversational AI Interface - Natural language chat interface for defining, refining, and generating application features
FR19: Kanban Board for Project Management - Visual drag-and-drop interface where users manage project phases, track feature progress, and organize development workflow
FR20: Agent Visualization - Real-time visual representation of Russian Doll agent hierarchy showing agent spawning, task decomposition, and delegation flow

### NonFunctional Requirements

NFR1: Local-First Design - All user data stored in SQLite database, no cloud accounts required, no telemetry by default
NFR2: Security - API keys encrypted at rest using AES-GCM with machine-based key derivation (argon2); OS keyring storage preferred, encrypted SQLite fallback with zeroize; Plugin sandboxing using WASM with CPU/memory limits and multi-tier verification; Offline license verification with ed25519 cryptographic signatures; Comprehensive audit logging for security events; Rate limiting (requests/minute, tokens/minute); Regular security key rotation; Semantic filter at embedding retrieval to prevent indirect prompt injection; Checkpoint file signing with ed25519 for integrity verification; Plugin signing ecosystem with three-tier trust model (built-in, verified, unverified)
NFR3: Performance - Progressive disclosure reduces token usage by ~10x through layered context summaries; Vector-based semantic search for skills and context (1536-dimensional embeddings); Efficient conflict detection using file checksums; Caching support for LLM prompts (prompt caching)
NFR4: Reliability - Network resilience with exponential backoff retry (max 3 retries, 30s max delay); Automatic model fallback (default → Haiku → GPT-4o); Database migrations for schema evolution; Recovery system with checkpoint-based rollback; Generation transaction tracking for cleanup on failure
NFR5: Scalability - Multi-project resource locking using async semaphores with timeouts; Cross-project search and context sharing (opt-in only); Session management with context window tracking; Plugin registry with hot-swappable capabilities
NFR6: Usability - Multiple interfaces (CLI, GUI, API) sharing same command library; Explicit control (no automatic git operations, no background processes); Clear error messages with actionable suggestions; Cost dashboard with per-model and per-feature breakdown
NFR7: Extensibility - Plugin system for framework-specific code generation; Lifecycle hooks for custom behavior at key events; Configurable model routing rules with RL optimization; Script-based hook handlers for external automation
NFR8: Accessibility - High contrast ratios maintained across all color combinations; Keyboard navigation support for all interactive elements; Screen reader announcements for state changes; Focus indicators and visible focus states; Reducing motion preference support (disable animations option)
NFR9: Responsiveness - Neural network scales from 600px max on desktop to 400px on tablet; Panel width adjusts from 400px → 320px → 100% (mobile); Stacked layout on mobile (neural panel top, info panel bottom)
NFR10: Theming Support - Design system built to support both dark (default) and light modes; CSS custom properties enable theming without component changes; System preference detection with manual override option
NFR11: Error Handling - Comprehensive error handling with domain-specific error enums; User-friendly error messages with actionable context; Graceful degradation when connectivity fails; Clear recovery options with one-click rollback
NFR12: Performance Targets - Card creation under 2 seconds after chat confirmation; Drag-and-drop at 60fps with no lag; Chat response under 5 seconds for simple confirmations, under 15 for complex clarifying questions; LLM connection timeouts with automatic retry and fallback
NFR13: Testing Strategy - Unit tests co-located with modules; Integration tests in tests/ directory; E2E tests for GUI; Comprehensive test coverage for security-critical components (encryption, signing, plugin sandbox)
NFR14: Security Scanning - Automated dependency security scanning (cargo-audit for Rust, npm audit for frontend); CI/CD pipeline enforces scanning with failure on high-severity vulnerabilities; Regular security audits for all external integrations
NFR15: Code Quality - SOLID principles applied throughout codebase; DDD structure with clear domain boundaries; Type safety across Rust ↔ TypeScript boundary via Tauri auto-generated types; Consistent naming conventions (snake_case for Rust/database, camelCase for TypeScript)

### Additional Requirements

**From Architecture:**
- Use create-tauri-app (React + TypeScript) as starter template for initialization
- SQLite vector extension: sqlite-vec (v0.1.6) for embedding-based search
- Cargo Workspace structure with 3 Crates: demiarch-core, demiarch-cli, demiarch-gui
- UI Framework: Tailwind CSS (v3.4) + shadcn/ui (Radix UI components)
- State Management: Zustand (v4.5.0) for React state
- Database Schema: SQLite per project (not shared) + JSONL for git sync
- API Key Storage: OS keyring (keyring crate) preferred, encrypted SQLite fallback with zeroize
- Encryption: AES-GCM for encryption, argon2 for key derivation, ChaChaRng for nonce generation
- Project Structure: DDD with domain/, application/, infrastructure/, interfaces/ layers
- Architecture Patterns: DDD + SOLID principles with trait-based design
- Rust Tech Stack: Tokio 1.41, sqlx 0.8, rusqlite 0.32, serde, uuid, chrono, reqwest 0.12, tauri 2.1, clap 4.5, ratatui 0.29, git2 0.19, thiserror 2.0, anyhow 1.0, wasmtime 27.0
- Security Crates: ed25519-dalek 2.1, aes-gcm 0.10, argon2 0.5, sha2 0.10, rand 0.8, zeroize 1.8, ring 0.17, governor 0.7
- Vector Extensions: Store 1536-dimensional embeddings as float[] in sqlite-vec
- Multi-Project: SQLite database per project for stronger isolation; Locks with timeouts + forced release on deadlock; Session tokens with short expiry, rotation on project switch

**From UX Design:**
- Custom design system based on neural network visualization (agents.html)
- Color palette: --bg-deep: #0d1b2a, --bg-mid: #1b263b, --bg-surface: #253346, --teal: #00f5d4, --magenta: #f72585, --amber: #ffc300
- Typography: IBM Plex Sans for UI, Fira Code for code/technical content
- Visual Language: Neural network visualization with concentric rings, animated nodes, flowing connections, particle animations, glassmorphism panels
- Agent Color Mapping: Teal for Orchestrator (Level 1), Magenta for Planner (Level 2), Amber for Workers/Coder/Reviewer/Tester (Level 3)
- Core Experience: Talk to Build (conversational AI), Drag to Track (kanban board)
- Dual-Audience Support: Technical developers and non-technical users with progressive complexity disclosure
- Design System Components: GlassPanel, AccentButton, Card, StatusIndicator, LogEntry, NeuralNetwork, AgentNode, ConnectionPath, KanbanBoard, KanbanColumn, KanbanCard, ChatContainer, MessageBubble, ChatInput, ConversationThread
- Responsiveness: Desktop (600px neural), Tablet (400px), Mobile (100% stacked)
- Animations: bg-pulse, node-pulse, connection-flow, particle-move; CSS transforms for GPU acceleration
- Accessibility: Reducing motion preference, keyboard navigation, screen reader support
- Automatic Behaviors: Checkpoint before major changes, progressive code disclosure, context window management, conflict detection

### FR Coverage Map

FR1: Epic 2 - Conversational Development (code generation)
FR2: Epic 2 - Conversational Development (document generation)
FR3: Epic 2 - Conversational Development (phase planning); Epic 3 - Project Management & Kanban (tracking)
FR4: Epic 3 - Project Management & Kanban (basic switching); Epic 6 - Multi-Project Workspace (full concurrency)
FR5: Epic 5 - Advanced Agent Capabilities (Russian Doll hierarchy)
FR6: Epic 5 - Advanced Agent Capabilities (progressive disclosure)
FR7: Epic 5 - Advanced Agent Capabilities (learned skills)
FR8: Epic 5 - Advanced Agent Capabilities (dynamic model routing)
FR9: Epic 4 - Code Safety & Recovery (conflict resolution)
FR10: Epic 7 - Plugin Ecosystem (plugin system)
FR11: Epic 1 - Foundation & Project Setup (storage); Epic 4 - Code Safety & Recovery (JSONL sync); Epic 8 - Offline & Operations (sync operations)
FR12: Epic 4 - Code Safety & Recovery (checkpoints, rollback)
FR13: Epic 6 - Multi-Project Workspace (session management); Epic 7 - Plugin Ecosystem (lifecycle hooks)
FR14: Epic 1 - Foundation & Project Setup (CLI/GUI interfaces)
FR15: Epic 1 - Foundation & Project Setup (basic tracking); Epic 8 - Offline & Operations (cost alerts)
FR16: Epic 8 - Offline & Operations (offline support)
FR17: Epic 1 - Foundation & Project Setup (API keys, encryption); Epic 7 - Plugin Ecosystem (plugin security)
FR18: Epic 2 - Conversational Development (chat interface)
FR19: Epic 3 - Project Management & Kanban (kanban board)
FR20: Epic 3 - Project Management & Kanban (basic status); Epic 5 - Advanced Agent Capabilities (full hierarchy)

## Epic List

### Epic 1: Foundation & Project Setup
Users can install Demiarch, create projects, and configure core infrastructure (storage, security, settings)
**FRs covered:** FR1 (initial setup), FR11 (Storage Pattern), FR17 (Security Infrastructure - API keys, encryption), FR14 (Multi-Interface Support - basic CLI/GUI), FR15 (Cost Management - basic tracking)
**User Outcome:** Users can launch Demiarch, create their first project, and have a working foundation

### Epic 2: Conversational Development
Users can describe features through natural language chat and have AI generate code, PRD/architecture documents, and break down features into phases
**FRs covered:** FR18 (Conversational AI Interface), FR1 (AI-Powered Code Generation), FR2 (Document Auto-Generation), FR3 (Phase Planning)
**User Outcome:** Users can talk to AI to define requirements and generate initial project structure and features

### Epic 3: Project Management & Kanban
Users can visualize their development progress through a drag-and-drop kanban board and track features through complete lifecycle
**FRs covered:** FR19 (Kanban Board), FR3 (Phase Planning - tracking), FR4 (Multi-Project - basic project switching), FR20 (Agent Visualization - basic status display)
**User Outcome:** Users can see all their features, track progress, and manage development workflow visually

### Epic 4: Code Safety & Recovery
Users can generate code safely with automatic checkpoints, rollback capability, and intelligent conflict resolution when AI regenerates code they've edited
**FRs covered:** FR12 (Recovery System), FR9 (Conflict Resolution), FR11 (Storage Pattern - JSONL sync)
**User Outcome:** Users can experiment freely with AI-generated code, knowing they can always recover their work and resolve conflicts

### Epic 5: Advanced Agent Capabilities
Users can leverage Russian Doll agent hierarchy for sophisticated code generation with progressive disclosure context management and learned skills for faster debugging
**FRs covered:** FR5 (Russian Doll Agent System), FR6 (Progressive Disclosure), FR7 (Learned Skills Extraction), FR8 (Dynamic Model Routing), FR20 (Agent Visualization - full hierarchy)
**User Outcome:** Users get faster, more intelligent code generation with visible agent workflow and reusable debugging knowledge

### Epic 6: Multi-Project Workspace
Users can work on 3-5 projects simultaneously with cross-project context sharing, unified sessions, and visual project switching
**FRs covered:** FR4 (Multi-Project Concurrency - full), FR13 (Lifecycle Hooks - session management)
**User Outcome:** Users can manage multiple active projects seamlessly without losing context between them

### Epic 7: Plugin Ecosystem
Users can install, verify, and use third-party plugins for additional frameworks and capabilities with WASM sandboxing and offline license verification
**FRs covered:** FR10 (Plugin System), FR17 (Security Infrastructure - plugin signing, sandboxing), FR13 (Lifecycle Hooks - plugin events)
**User Outcome:** Users can extend Demiarch's capabilities through a trusted, secure plugin ecosystem

### Epic 8: Offline & Operations
Users can work without connectivity with queued operations, automatic retry, and graceful degradation when offline
**FRs covered:** FR16 (Offline Support), FR15 (Cost Management - alerts), FR11 (Storage Pattern - sync operations)
**User Outcome:** Users can continue development even when internet connectivity is unreliable

---

## Epic 1: Foundation & Project Setup

Users can install Demiarch, create projects, and configure core infrastructure (storage, security, settings)

### Story 1.1: Initialize Project with Starter Template
As a new user,
I want to initialize a Demiarch project using create-tauri-app with React + TypeScript,
So that I can start development with a working foundation.

**Acceptance Criteria:**

**Given** Demiarch is installed and user runs initialization command
**When** User executes `demiarch init --name my-project --framework nextjs`
**Then** Project is created using create-tauri-app template with React + TypeScript
**And** Cargo workspace is configured with 3 crates: demiarch-core, demiarch-cli, demiarch-gui
**And** Project structure matches architecture specification (domain/application/infrastructure/)
**And** All dependencies from architecture are added to Cargo.toml
**And** User sees confirmation message: "Project 'my-project' initialized successfully"

### Story 1.2: Set Up SQLite Database per Project
As a user creating a project,
I want a local SQLite database created automatically for my project,
So that all my data stays local and is properly isolated.

**Acceptance Criteria:**

**Given** User has initialized a project and runs setup command
**When** Demiarch executes project setup
**Then** SQLite database file is created at `.demiarch/{project_id}/database.sqlite`
**And** Core tables are created: projects, user_preferences, metadata
**And** Schema version is recorded in metadata table as version 1
**And** Database file permissions are set to 0600 (owner read/write only)
**And** Connection pool is configured with 5 max connections
**And** User sees confirmation: "Database initialized for project 'my-project'"

### Story 1.3: Configure API Key Storage with Encryption
As a user,
I want to securely store my OpenRouter API key with encryption,
So that my credentials are protected and not exposed.

**Acceptance Criteria:**

**Given** User has an OpenRouter API key and Demiarch is running
**When** User enters API key via CLI command `demiarch config set-api-key` or GUI settings panel
**Then** System attempts to store key in OS keyring first (keyring crate)
**And** If OS keyring fails, stores in encrypted SQLite table (projects.openrouter_api_key_encrypted)
**And** Key is encrypted using AES-GCM with argon2-derived key from machine ID
**And** Nonce is generated using ChaChaRng (cryptographically secure, never reused)
**And** All plaintext containing key is zeroed from memory immediately after encryption (zeroize)
**And** User sees success message without key being displayed in logs or console

### Story 1.4: Implement Basic CLI Interface
As a developer user,
I want to interact with Demiarch through a command-line interface,
So that I can script operations and use terminal workflows.

**Acceptance Criteria:**

**Given** Demiarch is installed and database is initialized
**When** User runs `demiarch --help`
**Then** CLI displays available commands: project, generate, sync, config, watch
**And** Each command shows usage and required arguments
**When** User runs `demiarch project list`
**Then** All projects are displayed in a table format with ID, name, status, framework
**And** Empty state shows "No projects found. Create one with `demiarch project create`"
**And** Errors display with user-friendly messages and actionable suggestions

### Story 1.5: Implement Basic GUI Entry Point
As a non-technical user,
I want to launch a desktop application with a graphical interface,
So that I can interact with Demiarch visually without using the terminal.

**Acceptance Criteria:**

**Given** Demiarch is installed with GUI dependencies
**When** User launches `demiarch gui` or clicks desktop icon
**Then** Tauri window opens with default dark theme
**And** Window displays navigation header with project selector
**And** Window shows empty state with "Create your first project" prompt
**And** Window is resizable with minimum dimensions 800x600
**And** React app loads without JavaScript errors in console

### Story 1.6: Track Basic LLM Costs
As a user,
I want to see how much I'm spending on LLM calls,
So that I can manage my AI development budget.

**Acceptance Criteria:**

**Given** User has configured API key and has made at least one LLM request
**When** User runs `demiarch costs --project my-project`
**Then** System displays total cost spent for project in USD
**And** Cost breakdown shows tokens used per model (prompt + completion)
**And** Costs are retrieved from llm_usage table with accurate calculations
**When** User views in GUI, a dashboard panel shows daily cost vs budget limit
**And** Zero cost displays as "$0.00" not empty or null

---

## Epic 2: Conversational Development

Users can describe features through natural language chat and have AI generate code, PRD/architecture documents, and break down features into phases

### Story 2.1: Implement Chat Interface with Conversation Threading
As a user,
I want to chat with Demiarch's AI through a conversational interface with message history,
So that I can discuss my project requirements naturally and see our conversation context.

**Acceptance Criteria:**

**Given** User has created a project and opened the chat interface (CLI or GUI)
**When** User types a message and sends it
**Then** Message appears immediately in chat stream with user avatar and timestamp
**And** AI response appears within 5 seconds with typing indicator shown during processing
**And** All messages are saved to chat_messages table with project_id, role, content, token_count, model
**And** Chat scrolls automatically to latest message
**And** User can scroll up to see full conversation history

### Story 2.2: Integrate OpenRouter LLM Client
As a user,
I want Demiarch to communicate with LLMs through OpenRouter API,
So that I can use multiple AI models for code generation and conversations.

**Acceptance Criteria:**

**Given** User has configured API key and initiated a chat or code generation request
**When** Demiarch makes an LLM API call
**Then** Request is sent to OpenRouter API with proper headers (Authorization: Bearer {encrypted_key})
**And** Model is selectable from configured defaults (anthropic/claude-sonnet-4-20250514)
**And** Request includes conversation context from chat_messages table
**And** Response is parsed and saved to chat_messages table with token_count
**And** Cost is calculated and saved to llm_usage table
**And** Network errors trigger exponential backoff retry (max 3 retries, 30s max delay)
**And** Model fallback activates on failure (default → Haiku → GPT-4o)

### Story 2.3: Generate Code from Natural Language Description
As a user,
I want to describe a feature in natural language and have AI generate complete, working code,
So that I can build applications without writing code manually.

**Acceptance Criteria:**

**Given** User is in a chat conversation and describes a feature (e.g., "Add user authentication with login and logout")
**When** AI processes the request and generates code
**Then** Generated files are created in the project repository at specified paths
**And** Each file is recorded in generated_code table with feature_id, file_path, content, original_checksum, current_checksum
**And** Files are created using the project's configured framework (Next.js, React, Flutter, etc.)
**And** User sees message: "Generated 5 files for user authentication feature"
**And** User can run the generated code and it functions as described

### Story 2.4: Auto-Generate PRD and Architecture Documents
As a user,
I want to automatically generate PRD and Architecture documents based on our conversation,
So that I can have formal project documentation without writing it manually.

**Acceptance Criteria:**

**Given** User has discussed project requirements in chat and requests document generation
**When** AI generates PRD document
**Then** PRD.md is created in project root with sections: Overview, Functional Requirements, Non-Functional Requirements, Architecture, Database Schema
**And** Content reflects all requirements discussed in conversation
**When** AI generates Architecture document
**Then** architecture.md is created in project root with sections: System Overview, Tech Stack, Database Schema, API Contracts
**And** Technical decisions from architecture are documented
**And** Both documents are saved to project_documents table with doc_type='prd' or 'architecture'

### Story 2.5: Break Down Features into Phases with User Stories
As a user,
I want to automatically break down my project into phases with user stories and acceptance criteria,
So that I can track development progress in a structured way.

**Acceptance Criteria:**

**Given** User has discussed project scope and requests phase planning
**When** AI analyzes requirements and generates phase breakdown
**Then** Phase records are created in phases table with name, description, status, order_index
**And** Feature records are created in features table linked to phase_id with name, description, acceptance_criteria, priority
**And** User sees organized phases: "Discovery", "Planning", "Building", "Complete"
**And** Each phase shows count of features within it
**And** User can drag features between phases (in kanban board, Epic 3)

---

## Epic 3: Project Management & Kanban

Users can visualize their development progress through a drag-and-drop kanban board and track features through complete lifecycle

### Story 3.1: Create Kanban Board Layout with Columns
As a user,
I want to see a kanban board with columns representing project phases (Discovery, Planning, Building, Complete),
So that I can visualize my project's feature workflow.

**Acceptance Criteria:**

**Given** User has created a project with phases from Epic 2
**When** User opens kanban board in GUI
**Then** Board displays with 4 columns: "Discovery", "Planning", "Building", "Complete"
**And** Each column shows feature cards linked to that phase (from features table)
**And** Column header displays feature count (e.g., "Planning (3)")
**And** Layout uses glassmorphism panels with custom design system colors
**And** Empty columns show "No features in this phase" message
**And** Board is responsive: 4 columns on desktop, 2 on tablet, 1 on mobile

### Story 3.2: Implement Feature Cards with Drag-and-Drop
As a user,
I want to drag feature cards between kanban columns to update their status,
So that I can track progress visually with satisfying interactions.

**Acceptance Criteria:**

**Given** Kanban board is displaying features
**When** User drags a card and hovers over a column
**Then** Column highlights with teal glow indicating drop target
**When** User releases card over valid column
**Then** Card snaps into column with smooth animation (60fps, no lag)
**And** Feature's phase_id in database is updated to new column's phase
**And** Card shows updated status badge reflecting new phase
**And** Update happens within 2 seconds of drop
**And** User can cancel drag with Escape key or by clicking outside board

### Story 3.3: Display Feature Details on Card Expansion
As a user,
I want to expand a kanban card to see feature details, requirements, and linked conversation,
So that I can review what was discussed without navigating away.

**Acceptance Criteria:**

**Given** Kanban board is displaying features
**When** User clicks on a card
**Then** Modal/panel opens showing: feature name, description, acceptance_criteria, priority
**And** "View Conversation" button links to chat history that created this feature
**And** "View Code" button shows generated files for this feature
**And** Modal closes with Escape key or clicking outside
**And** Details are read-only but user can navigate to related views

### Story 3.4: Basic Project Switching UI
As a user with multiple projects,
I want to switch between projects via a dropdown or keyboard shortcuts,
So that I can work on different projects without restarting Demiarch.

**Acceptance Criteria:**

**Given** User has created multiple projects and is in GUI
**When** User clicks project dropdown or presses Cmd/Ctrl + 1-5
**Then** Dropdown shows list of all projects with names and status
**And** Selecting a project loads that project's data (phases, features, chat)
**And** Active project is visually indicated with teal accent in navigation
**And** Shortcut Cmd/Ctrl + 1-5 switches to projects 1-5 if they exist
**And** Current project state is preserved (active chat, kanban view, focused card)
**And** Switch completes within 1 second

### Story 3.5: Basic Agent Status Visualization
As a user,
I want to see basic agent activity status when code generation is running,
So that I know what's happening without technical details.

**Acceptance Criteria:**

**Given** User has triggered code generation for a feature
**When** Agents are working on a feature
**Then** Panel shows simplified status: "AI agents working on this feature"
**And** Status pulses with amber color to indicate activity
**When** Generation completes
**Then** Status changes to "Feature complete" with teal color
**And** Panel shows summary: "Generated 5 files, 3 components, 2 API routes"
**When** Generation fails
**Then** Status shows "Generation failed - see logs" with error message
**And** Panel offers "Retry" button

---

## Epic 4: Code Safety & Recovery

Users can generate code safely with automatic checkpoints, rollback capability, and intelligent conflict resolution when AI regenerates code they've edited

### Story 4.1: Create Automatic Checkpoints Before Major Changes
As a user,
I want Demiarch to automatically create a complete project state checkpoint before generating new code,
So that I can always recover if something goes wrong.

**Acceptance Criteria:**

**Given** User triggers code generation for a feature
**When** Generation starts
**Then** System creates checkpoint record in checkpoints table with project_id, feature_id, description, snapshot_data
**Then** snapshot_data contains: JSON serialization of current project state (phases, features, chat_messages, generated_code)
**Then** Checkpoint size in bytes is recorded
**Then** Checkpoint is signed with ed25519 private key, signature stored
**When** User views checkpoint list
**Then** Checkpoints show: timestamp, description (e.g., "Before generating User Auth"), size
**And** Oldest checkpoints beyond retention limit are automatically deleted

### Story 4.2: Implement Rollback to Previous Checkpoint
As a user,
I want to restore my project to a previous checkpoint with one click,
So that I can recover from mistakes or failed generations.

**Acceptance Criteria:**

**Given** User has one or more checkpoints and opens recovery UI
**When** User selects a checkpoint and clicks "Restore"
**Then** System verifies checkpoint signature using ed25519 public key
**And** If signature is invalid, user sees error: "Checkpoint signature verification failed"
**When** Signature is valid
**Then** snapshot_data is deserialized and applied to project state
**And** All tables (phases, features, chat_messages, generated_code) are restored from snapshot
**And** Current generated code files are reverted to checkpoint versions
**And** User sees success: "Project restored to state from [timestamp]"
**And** Restore completes within 5 seconds
**And** Checkpoint creation happens after restore (for safety)

### Story 4.3: Detect User Edits to Generated Code
As a user,
I want Demiarch to automatically detect when I edit AI-generated files,
So that conflicts are flagged when AI regenerates that code.

**Acceptance Criteria:**

**Given** User has generated code files recorded in generated_code table
**When** User edits a file in their editor
**Then** Demiarch watch system detects file change (via filesystem watcher)
**And** New checksum is calculated for file (SHA-256)
**And** generated_code.current_checksum is updated with new checksum
**And** generated_code.user_modified is set to 1
**And** Record is added to user_code_edits table with edit_type='modify', new_content, detected_at
**When** User views code in GUI
**Then** Modified files show visual indicator (e.g., amber dot or "Edited" badge)

### Story 4.4: Display Conflict Resolution Interface
As a user,
I want to see side-by-side diff when AI regenerates code I've edited,
So that I can choose what to keep intelligently.

**Acceptance Criteria:**

**Given** User has modified a generated file and AI regenerates it
**When** Regeneration completes
**Then** System detects conflict by comparing checksums
**And** generated_code.conflict_status is set to 'pending'
**When** User views feature in GUI
**Then** Modal opens with side-by-side diff: original AI version vs new AI version
**And** User's edits are highlighted with amber overlay
**And** Options displayed: "Keep My Changes", "Keep AI Version", "Merge Both", "Manual Edit"
**And** Security annotations show for suspicious additions (e.g., "adds exec()", "adds eval()")
**When** User selects "Keep My Changes"
**Then** New AI code is discarded, user's version is kept
**And** conflict_status is set to 'resolved_keep_user'
**And** Record is added to conflict_resolutions table

### Story 4.5: Export to JSONL for Git Sync
As a user,
I want to explicitly export my project data to JSONL format for git version control,
So that I can commit my project history without automatic sync.

**Acceptance Criteria:**

**Given** User has a project and wants to sync to git
**When** User runs `demiarch sync --flush-only`
**Then** All tables are exported to .demiarch/{project_id}/export.jsonl
**And** Each record is one JSON line in JSONL format
**And** metadata.dirty flag is set to 0
**And** User sees: "Exported to export.jsonl. Run 'git add .demiarch/ && git commit' to save."
**When** User runs `demiarch sync --import-only`
**Then** Data from export.jsonl is imported to SQLite database
**And** Each line is validated against JSON schema before import
**And** Import completes without data corruption
**And** User sees: "Imported N records from export.jsonl"

---

## Epic 5: Advanced Agent Capabilities

Users can leverage Russian Doll agent hierarchy for sophisticated code generation with progressive disclosure context management and learned skills for faster debugging

### Story 5.1: Implement Russian Doll Agent Hierarchy
As a user,
I want Demiarch to use a 3-level agent hierarchy (Orchestrator → Planner → Coder/Reviewer/Tester) for code generation,
So that complex features are broken down and delegated intelligently.

**Acceptance Criteria:**

**Given** User triggers code generation for a complex feature
**When** Orchestrator agent (Level 1) receives request
**Then** Orchestrator spawns a Planner agent (Level 2) via AgentTool call
**Then** Planner decomposes feature into tasks (e.g., "Create UI components", "Implement API routes")
**And** Planner spawns Coder, Reviewer, and Tester agents (Level 3) for each task
**And** All agent executions are recorded in agent_executions table with parent_agent_id, agent_type, status
**When** Coder agent completes code generation
**Then** Result is returned to Planner
**When** Reviewer validates and Tester creates tests
**Then** All results bubble up through hierarchy to Orchestrator
**And** User sees: "Orchestrator → Planner → Coder, Reviewer, Tester" status
**And** Generation completes with all tasks finished

### Story 5.2: Implement Progressive Disclosure for Context Management
As a user,
I want Demiarch to automatically retrieve only the most relevant context to save tokens,
So that my conversations are efficient and cost-effective.

**Acceptance Criteria:**

**Given** User has a long conversation history and makes a new request
**When** System prepares context for LLM
**Then** System retrieves layered summaries from context_summaries table
**And** Summary types follow: detail_level=1 (index), 2 (timeline), 3 (full)
**And** Only relevant summaries are included based on semantic similarity (using sqlite-vec embeddings)
**And** Full context is retrieved only when needed (user explicitly requests more detail)
**When** Context is prepared
**Then** Total token count is displayed to user
**And** User sees: "Using 3K tokens from conversation (10x savings via progressive disclosure)"
**And** If context would exceed limit, system creates automatic summary

### Story 5.3: Extract and Store Learned Skills
As a user,
I want Demiarch to automatically extract debugging solutions as reusable skills,
So that similar problems are solved faster in the future.

**Acceptance Criteria:**

**Given** An agent encounters and solves a technical problem (e.g., "Fixed TypeScript error with async/await")
**When** Agent completes successfully
**Then** System analyzes error and solution for reusability
**And** If solution is reusable, a learned_skills record is created
**And** Fields populated: name, description, problem, trigger_conditions (JSON array), solution, verification, category='debugging'
**And** Quality score is initialized to 0.0
**And** Embedding is generated for solution (1536-dimensional using OpenAI text-embedding-3-small)
**And** Embedding is stored in skill_embeddings table with embedding_model
**When** User views learned skills
**Then** Skills display: name, description, problem, usage_count, quality_score
**And** User can mark skill as "Verified" to increase quality score

### Story 5.4: Implement Semantic Search for Skills Retrieval
As a user,
I want Demiarch to automatically suggest relevant learned skills when I encounter a problem,
So that I can apply proven solutions without waiting for AI.

**Acceptance Criteria:**

**Given** User has learned skills stored and encounters a new error
**When** Error is detected or user describes problem in chat
**Then** System generates embedding for error/problem description
**And** System queries skill_embeddings table for KNN search (sqlite-vec)
**And** Top 3 most similar skills are returned (by cosine distance)
**Then** System applies semantic filter to retrieved skills (scans for prompt injection patterns)
**If** filter flags suspicious content
**Then** Skill is not suggested, event logged in audit_log with event_type='prompt_injection_attempt'
**When** No injection detected
**Then** Skills are suggested to user: "Similar problem found in 3 skills. Apply this solution?"
**And** User can click to apply skill or decline suggestion
**And** When skill is applied, activation_type='automatic' is recorded in skill_activations table

### Story 5.5: Implement Dynamic Model Routing with RL
As a user,
I want Demiarch to automatically select the best model for each task type,
So that I get optimal performance at lowest cost.

**Acceptance Criteria:**

**Given** User triggers a task (code generation, debugging, math calculation)
**When** System determines task_type
**Then** System queries model_routing_rules table for matching task types
**And** Priority-ordered models are returned based on: is_specialized flag, min_quality_score, max_cost_per_1k
**When** Model is selected
**Then** Selection is recorded in routing_decisions table with: agent_execution_id, task_type, selected_model, selection_reason
**And** Alternatives considered are stored as JSON array
**When** Task completes
**Then** Performance metrics are recorded in model_performance table: model_id, task_type, outcome (success/failure/partial), quality_score, latency_ms, tokens_used, cost_usd
**And** Model performance scores are updated based on outcomes (RL feedback loop)
**And** Routing rules may be adjusted automatically based on learned performance

### Story 5.6: Full Agent Visualization with Hierarchy Tree
As a technical user,
I want to see complete Russian Doll agent execution tree with detailed metrics,
So that I can understand what each agent did and optimize performance.

**Acceptance Criteria:**

**Given** User has triggered code generation and wants to inspect agent execution
**When** User opens "Agent Execution" panel or clicks on agent status
**Then** Neural network visualization displays with concentric rings:
  - Outer ring: Orchestrator (teal)
  - Middle ring: Planner (magenta)
  - Inner ring: Coder, Reviewer, Tester (amber)
**And** Each agent node shows: status, execution time, token usage, cost
**And** Connections between nodes animate with flowing particles
**When** User clicks on an agent node
**Then** Detail panel shows: agent_type, input_context, output_result, error_message (if failed)
**And** User can see full execution tree with parent-child relationships
**And** Technical details are shown in readable format (e.g., "Coder generated 238 tokens in 3.2s")

---

## Epic 6: Multi-Project Workspace

Users can work on 3-5 projects simultaneously with cross-project context sharing, unified sessions, and visual project switching

### Story 6.1: Implement Global Session Management
As a user,
I want to have a global session that tracks active projects and context windows,
So that I can work across multiple projects without losing state.

**Acceptance Criteria:**

**Given** User opens Demiarch
**When** Session starts
**Then** Global session record is created in global_sessions table with status='active', started_at
**And** current_project_id is set to last used project or null
**And** active_project_ids JSON array tracks all open projects
**And** context_data JSON stores context windows for each project
**When** User switches between projects
**Then** current_project_id is updated to new project
**And** context windows are preserved and restored for each project
**And** Session persists in database across GUI/CLI restarts

### Story 6.2: Implement Resource Locking for Concurrency
As a user with multiple projects,
I want agents to acquire locks on resources (files, database, LLM, features) to prevent conflicts,
So that concurrent operations don't corrupt data.

**Acceptance Criteria:**

**Given** Multiple agents are running across projects
**When** Agent needs to access a resource (e.g., write to file)
**Then** Agent calls LockManager::acquire with resource_key (project_id, resource_type, resource_key)
**And** Lock is granted if resource is free or timeout occurs after 300 seconds
**And** Lock is recorded in resource_locks table with agent_id, acquired_at
**And** If lock is held by another agent, request blocks until timeout
**When** Agent completes operation
**Then** Lock is released via LockGuard::release or Drop implementation
**And** resource_locks.released_at is set to current time
**And** If deadlock is detected, lock is force-released after timeout
**And** Warning is logged: "Deadlock detected on resource {resource_key}"

### Story 6.3: Cross-Project Search with Opt-In
As a user with opt-in enabled,
I want to search across multiple projects to find code patterns and reuse solutions,
So that I can leverage work from other projects.

**Acceptance Criteria:**

**Given** User has enabled cross-project search in settings and has 2+ projects
**When** User enters search query in GUI or CLI
**Then** System searches selected projects' features, chat_messages, generated_code tables
**Then** Results show project name, feature name, relevance score
**And** User sees which project each result came from
**When** User clicks on a result
**Then** Context is loaded from source project
**And** Reference is created in cross_project_refs table with source_project_id, target_project_id, ref_type
**And** User can navigate to source project to view full details
**And** Search can be disabled per-project in settings

### Story 6.4: Session Recovery on Restart
As a user,
I want my session state to be restored automatically when I restart Demiarch,
So that I can continue working without setup.

**Acceptance Criteria:**

**Given** User had an active session with projects and context windows
**When** User reopens Demiarch after restart
**Then** System queries global_sessions table for last active session
**And** Session state is reconstructed from context_data JSON
**And** current_project_id is restored
**And** All active projects are loaded into memory
**And** GUI displays projects in their last viewed state
**And** User sees: "Welcome back! Restored session with 3 active projects"

### Story 6.5: Session End and Cleanup
As a user,
I want to explicitly end my session with cleanup,
So that resources are released and state is saved properly.

**Acceptance Criteria:**

**Given** User is in an active session
**When** User closes Demiarch or clicks "End Session"
**Then** global_sessions.status is updated to 'completed'
**And** completed_at is set to current time
**And** All resource locks for session are released
**And** Context windows are serialized to context_data before cleanup
**And** Audit log entry is created: event_type='session_end', details={session_id}
**And** Session cleanup completes within 2 seconds

---

## Epic 7: Plugin Ecosystem

Users can install, verify, and use third-party plugins for additional frameworks and capabilities with WASM sandboxing and offline license verification

### Story 7.1: Install Built-in Framework Plugins
As a user,
I want Demiarch to include built-in plugins for major frameworks (Next.js, React Native, Flutter, etc.),
So that I can generate code for my preferred framework without manual setup.

**Acceptance Criteria:**

**Given** User selects a framework during project creation or generation
**When** Framework is a built-in plugin (Next.js, Vue 3, Flutter, etc.)
**Then** Plugin is loaded from installed_plugins table with source='builtin'
**Then** Plugin metadata is displayed: name, version, capabilities, api_version
**And** Plugin can generate components, API routes, and models for that framework
**And** User sees: "Using built-in plugin: Next.js v14"

### Story 7.2: Load and Execute WASM Plugins
As a user,
I want to install and use third-party plugins compiled to WASM,
So that I can extend Demiarch's capabilities securely.

**Acceptance Criteria:**

**Given** User has a WASM plugin file and installs it
**When** Plugin is installed
**Then** installed_plugins record is created with source='local' or 'registry', source_url, checksum
**Then** Plugin is loaded into Wasmtime sandbox with configured capabilities (read_files, write_files, network, max_memory_mb, max_cpu_seconds)
**And** Fuel limit is enforced (10M fuel max, randomized to prevent side-channel attacks)
**And** Only allowed capabilities are linked (plugin_func_wrap for read, write)
**And** Plugin execution is logged with fuel consumption metrics
**When** Plugin exceeds limits or crashes
**Then** Execution is terminated and error is returned to user
**And** Fuel consumption outliers are flagged for security review

### Story 7.3: Offline License Verification with ed25519
As a user,
I want Demiarch to verify plugin licenses offline using ed25519 signatures,
So that I can use plugins without internet connectivity.

**Acceptance Criteria:**

**Given** User installs a plugin with licensing
**When** Plugin is installed
**Then** plugin_licenses record is created with plugin_id, license_key, license_type, tier, issued_at, expires_at
**And** Signature is verified using public key compiled into binary (const array)
**Then** If signature is invalid, validation_status is set to 'invalid' and user sees error
**When** Signature is valid
**Then** validation_status is set to 'valid', validated_at is set to current time
**And** If license has expired, validation_status is 'expired' and user sees: "License expired on {expires_at}"
**And** Plugin functionality is available based on tier (free, paid features enabled)

### Story 7.4: Implement Three-Tier Plugin Trust Model
As a user,
I want to see plugin trust levels (built-in, verified third-party, unverified) with appropriate warnings,
So that I can make informed decisions about plugin safety.

**Acceptance Criteria:**

**Given** User views plugins in GUI or runs plugin list command
**When** Plugins are displayed
**Then** Each plugin shows trust badge:
  - "Built-in" (green checkmark) - fully trusted
  - "Verified" (blue shield) - signed with ed25519, third-party
  - "Unverified" (amber warning) - no signature, sandboxed only
**And** Unverified plugins show warning: "This plugin is unverified. Use at your own risk."
**And** Verified plugins show: "Signed by {publisher} verified at {date}"
**And** Built-in plugins show: "Official Demiarch plugin"
**And** User can filter plugins by trust level

### Story 7.5: Configure Plugin Capabilities and Permissions
As a user,
I want to review and approve plugin capabilities before first use,
So that I understand what a plugin can access.

**Acceptance Criteria:**

**Given** User installs a plugin with capabilities configured
**When** User first uses the plugin
**Then** Modal displays requested capabilities: read_files (yes/no), write_files (yes/no), network (yes/no), execute_commands (yes/no)
**And** Each capability shows plain-language explanation (e.g., "read_files: This plugin can read your project files")
**And** User can approve all capabilities or select specific ones
**And** Denied capabilities are not linked to WASM sandbox
**And** User's choice is saved in plugin_config table per capability
**And** If user denies all capabilities, plugin is disabled with message: "Plugin requires permissions to function"

### Story 7.6: Implement Plugin Pricing Models
As a user,
I want Demiarch to support multiple plugin pricing models (free, paid, usage-based, trial, watermark),
So that I can choose plugins with my preferred pricing.

**Acceptance Criteria:**

**Given** User installs a plugin with pricing_model specified
**When** Plugin pricing model is loaded
**Then** installed_plugins.pricing_model is set to: 'free', 'paid', 'usage', 'trial', or 'watermark'
**When** pricing_model is 'paid'
**Then** User is prompted to enter license_key during installation
**When** pricing_model is 'usage'
**Then** plugin_usage records are tracked per plugin with usage_count, period_start, period_end
**And** When free_quota is exceeded, user sees: "Usage limit exceeded. Upgrade to continue."
**When** pricing_model is 'trial'
**Then** User sees trial days remaining and expiration date
**When** pricing_model is 'watermark'
**Then** Generated code includes visible watermark: "Created with {PluginName}"
**And** Watermark removal prompts: "Remove watermark by purchasing full license"

### Story 7.7: Lifecycle Hooks for Plugin Events
As a plugin developer or power user,
I want plugins to register handlers for lifecycle events (session_start, pre_generation, post_generation, on_error, on_checkpoint),
So that plugins can react to Demiarch events.

**Acceptance Criteria:**

**Given** Plugin is loaded and configured
**When** Demiarch triggers lifecycle event (e.g., pre_generation)
**Then** System queries lifecycle_hooks table for handlers with hook_type matching event
**Then** Each handler is executed based on handler_type: 'internal', 'plugin', 'script'
**When** handler_type is 'plugin'
**Then** Handler config JSON includes plugin_id and invokes plugin function
**When** handler_type is 'script'
**Then** Handler config JSON includes script_path and executes external script
**When** Handler completes
**Then** Record is added to hook_executions table with hook_id, session_id, trigger_context, result, duration_ms
**And** If handler fails, error is logged and event continues
**And** Handler priority is respected (lower number = higher priority)

---

## Epic 8: Offline & Operations

Users can work without connectivity with queued operations, automatic retry, and graceful degradation when offline

### Story 8.1: Detect Network Connectivity and Enter Offline Mode
As a user,
I want Demiarch to automatically detect when internet is unavailable,
So that the system degrades gracefully without errors.

**Acceptance Criteria:**

**Given** Demiarch is running and making LLM requests
**When** Network connectivity is lost (timeout, DNS failure, no route)
**Then** System sets offline_mode=true and displays degraded UI banner
**And** User sees message: "Offline mode activated. Operations will queue until connection restores."
**And** All pending LLM requests are queued to queued_operations table
**When** Connectivity is detected again
**Then** System exits offline mode and processes queued operations
**And** User sees: "Connection restored. Processing 3 queued operations."

### Story 8.2: Queue Operations During Offline Mode
As a user,
I want my operations to be automatically queued when I'm offline,
So that I can continue working and have operations executed when connectivity returns.

**Acceptance Criteria:**

**Given** User is offline and performs an action requiring network (LLM call, sync, plugin install)
**When** Operation is initiated
**Then** Record is created in queued_operations table with operation_type, payload JSON, status='pending'
**And** retry_count is initialized to 0, max_retries set to 3
**When** User views operations queue
**Then** Queue displays count of pending operations
**And** Each operation shows type and status (pending/processing/completed/failed)
**And** User can cancel individual operations with confirmation

### Story 8.3: Automatic Retry on Connectivity Restore
As a user,
I want queued operations to automatically retry with exponential backoff when connection returns,
So that I don't need to manually trigger each failed operation.

**Acceptance Criteria:**

**Given** System has queued operations and detects connectivity restored
**When** Queue processing starts
**Then** Each pending operation is processed in FIFO order
**When** Operation fails
**Then** retry_count is incremented
**And** Exponential backoff is applied: delay = 2^retry_count * base_delay (max 30 seconds)
**When** retry_count reaches max_retries (3)
**Then** Operation status is set to 'failed'
**And** last_error is recorded in queued_operations table
**And** User is notified: "Operation '{operation_type}' failed after 3 retries. See logs."
**When** Operation succeeds
**Then** status is set to 'completed', processed_at is set to current time
**And** User sees completion notification

### Story 8.4: Graceful Degradation for Offline Features
As a user,
I want non-essential features to be disabled when offline,
So that I can continue core development without confusion.

**Acceptance Criteria:**

**Given** User is offline
**When** System enters degraded mode
**Then** LLM-dependent features show "Unavailable offline" message with amber warning
**And** Offline-capable features remain fully functional (file editing, project viewing, checkpoint management)
**When** User attempts offline feature
**Then** User sees clear explanation: "This feature requires internet connectivity. Your operation has been queued."
**And** Queue confirmation is shown: "Operation queued. Will run when online."
**When** Connectivity restores
**Then** All features are re-enabled without requiring restart

### Story 8.5: Cost Alerts with Daily Limits
As a user,
I want to be alerted when approaching or exceeding daily cost limits,
So that I can control spending and avoid unexpected bills.

**Acceptance Criteria:**

**Given** User has set cost_daily_limit_usd in user_preferences (default $10.00)
**When** System tracks LLM usage throughout the day
**Then** Daily cost is calculated from llm_usage table for current date
**And** When cost reaches 80% of limit (cost_alert_threshold)
**Then** cost_alerts record is created with alert_type='daily_threshold'
**Then** User sees notification: "Daily cost alert: $8.00 of $10.00 spent. 20% remaining."
**When** cost reaches or exceeds daily_limit
**Then** cost_alerts record is created with alert_type='daily_limit'
**Then** User sees blocking message: "Daily cost limit reached. Generate blocked until tomorrow or increase limit."
**And** cost_alerts.acknowledged is set to 0
**When** User acknowledges alert
**Then** acknowledged is set to 1 and notification dismisses
**And** User can view cost history with daily breakdowns

### Story 8.6: Project Rate Limits and Resource Throttling
As a user,
I want Demiarch to enforce project-specific rate limits (hourly requests, concurrent agents),
So that resource usage stays controlled and predictable.

**Acceptance Criteria:**

**Given** User has configured rate limits or uses defaults
**When** User initiates operations (LLM requests, file operations)
**Then** System checks project_rate_limits table for limit_type (hourly_requests, concurrent_agents)
**And** Current value is tracked and incremented
**When** current_value reaches limit_value
**Then** Operation is throttled with message: "Rate limit exceeded. Wait {reset_at} to continue."
**When** reset_at time passes
**Then** current_value is reset to 0
**When** User views project settings
**Then** Rate limits show: type, current_value, limit_value, reset_at
**And** User can adjust limits (not below system minimums)
**And** Limits persist across sessions

---

**Epic 8 Summary:** 6 stories created covering offline detection, operation queuing, automatic retry, graceful degradation, cost alerts, and rate limiting

---

## All Epics Complete

**Summary:**
- **8 Epics** with complete breakdown
- **38 Stories** across all epics
- **All 20 FRs** covered by stories
- **All 15 NFRs** addressed
- **Standalone epics** - each delivers complete user value
- **Dependency-free within epics** - each story can be completed independently

**FR Coverage Summary:**
- Epic 1: Foundation & Project Setup (6 stories) - FR1, FR11, FR17, FR14, FR15
- Epic 2: Conversational Development (5 stories) - FR18, FR1, FR2, FR3
- Epic 3: Project Management & Kanban (5 stories) - FR19, FR3, FR4, FR20
- Epic 4: Code Safety & Recovery (5 stories) - FR12, FR9, FR11
- Epic 5: Advanced Agent Capabilities (6 stories) - FR5, FR6, FR7, FR8, FR20
- Epic 6: Multi-Project Workspace (5 stories) - FR4, FR13
- Epic 7: Plugin Ecosystem (7 stories) - FR10, FR17, FR13
- Epic 8: Offline & Operations (6 stories) - FR16, FR15, FR11

**All requirements from PRD, Architecture, and UX Design have been decomposed into actionable stories ready for implementation.**

