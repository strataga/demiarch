# Story 1.1: Initialize Project with Starter Template

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a new user,
I want to initialize a Demiarch project using create-tauri-app with React + TypeScript,
so that I can start development with a working foundation.

## Acceptance Criteria

1. **Given** Demiarch is installed and user runs initialization command
2. **When** User executes `demiarch init --name my-project --framework nextjs`
3. **Then** Project is created using create-tauri-app template with React + TypeScript
4. **And** Cargo workspace is configured with 3 crates: demiarch-core, demiarch-cli, demiarch-gui
5. **And** Project structure matches architecture specification (domain/application/infrastructure/)
6. **And** All dependencies from architecture are added to Cargo.toml
7. **And** User sees confirmation message: "Project 'my-project' initialized successfully"

## Tasks / Subtasks

- [x] Run create-tauri-app initialization command (AC: 3)
  - [x] Execute: `npm create tauri-app@latest demiarch --template react-ts`
  - [x] Verify generated project structure
  - [x] Confirm React + TypeScript template used
- [x] Configure Cargo workspace with 3 crates (AC: 4)
  - [x] Create workspace Cargo.toml in project root
  - [x] Create demiarch-core/Cargo.toml
  - [x] Create demiarch-cli/Cargo.toml
  - [x] Create demiarch-gui/Cargo.toml
  - [x] Add workspace members to root Cargo.toml
- [x] Implement DDD directory structure (AC: 5)
  - [x] Create demiarch-core/src/domain/ directory
  - [x] Create demiarch-core/src/application/ directory
  - [x] Create demiarch-core/src/infrastructure/ directory
  - [x] Create demiarch-core/src/interfaces/ directory
- [x] Add architecture-specified dependencies (AC: 6)
  - [x] Add tokio to demiarch-core/Cargo.toml
  - [x] Add sqlx with sqlite + runtime-tokio features to demiarch-core/Cargo.toml
  - [x] Add rusqlite with bundled feature to demiarch-core/Cargo.toml
  - [x] Add serde, serde_json, uuid, chrono to demiarch-core/Cargo.toml
  - [x] Add reqwest to demiarch-core/Cargo.toml
  - [x] Add tauri to demiarch-gui/Cargo.toml
  - [x] Add clap to demiarch-cli/Cargo.toml
  - [x] Add ratatui to demiarch-cli/Cargo.toml
  - [x] Add thiserror and anyhow to all crates
- [x] Configure Tauri entry point (AC: N/A - template provides)
  - [x] Verify src-tauri/main.rs exists
  - [x] Verify src/ entry point with React
- [x] Verify project builds successfully (AC: implicit)
  - [x] Run `cargo check --workspace` across all crates
  - [x] Run `cargo build --workspace`
  - [x] Fix any compilation errors
- [x] Display success message (AC: 7)
  - [x] Print: "Project 'my-project' initialized successfully"
  - [x] Include next steps guidance

## Dev Notes

### Story Requirements Analysis

**Business Context:**
- Foundational story enabling all subsequent development
- Sets up workspace structure, core library, and shared command patterns
- Establishes technical foundation for CLI, GUI, and API interfaces

**Technical Dependencies:**
- Requires create-tauri-app CLI tool (npm package)
- Requires npm for package management
- Requires Rust toolchain (cargo, rustc)
- Requires Node.js for create-tauri-app execution

**Implementation Notes:**
- This is the FIRST story in Epic 1 - establishes project foundation
- No previous stories to reference - this is clean slate implementation
- All architecture decisions from lines 396-418 (starter template selection) and 406-417 (custom architecture) apply
- Follow naming conventions from Architecture.md lines 661-710 (database, API, code naming)
- Follow DDD structure from Architecture.md lines 597-662 (domain/application/infrastructure layers)

**Risk Mitigation:**
- Template provides working base but lacks demiarch-specific customizations
- Must follow architecture decisions precisely to avoid refactoring in later stories
- Document all deviations from template for future reference

### Technical Stack Requirements

**Backend (Rust):**
- Rust version: 1.41+
- Async runtime: Tokio 1.41 with "full" features
- Database: SQLite via rusqlite 0.32 (bundled), sqlx 0.8 (sqlite + runtime-tokio)
- Serialization: serde 1.0 with derive, serde_json 1.0
- HTTP: reqwest 0.12 with json + rustls-tls features
- CLI: clap 4.5 with derive, ratatui 0.29
- Error handling: thiserror 2.0, anyhow 1.0
- Utilities: uuid 1.11 (v4 + serde), chrono 0.4 (serde), hex 0.4, base64 0.22

**Frontend (React + TypeScript):**
- React: 18.3.0+
- TypeScript: 5.6.0+
- Tauri: 2.1.0
- Tauri API: @tauri-apps/api 2.1.0+

**Source:**
- PRD.md lines 104-143 (technology stack specification)
- Architecture.md lines 396-460 (starter template selection and rationale)
- Architecture.md lines 948-996 (tech stack with versions)

## Developer Context

### Guardrails and Constraints

**CRITICAL: Story 1.1 establishes the foundation - DO NOT deviate from architecture!**

**Architecture Compliance (MUST FOLLOW):**
1. **Workspace Structure MUST match Architecture.md lines 597-662:**
   - Cargo workspace with 3 members: demiarch-core, demiarch-cli, demiarch-gui
   - Root Cargo.toml contains [workspace] section with members array
   - Use resolver = "2" for consistent dependency resolution

2. **DDD Structure MUST match Architecture.md lines 577-622:**
   - demiarch-core MUST have domain/, application/, infrastructure/, interfaces/ directories
   - Each domain directory contains specific subdomains (projects, agents, context, skills, etc.)
   - Application layer contains use cases, NOT domain logic
   - Infrastructure layer contains external integrations (SQLite, LLM, Git, WASM)

3. **Dependencies MUST match Architecture.md lines 96-143:**
   - Use EXACT versions specified in tech stack
   - Add tokio with features = ["full"] to demiarch-core
   - Add sqlx with features = ["sqlite", "runtime-tokio"] to demiarch-core
   - Add rusqlite with features = ["bundled"] to demiarch-core
   - Add all serde, uuid, chrono, reqwest, thiserror, anyhow as specified
   - Add tauri to demiarch-gui
   - Add clap and ratatui to demiarch-cli

4. **Naming Conventions MUST match Architecture.md lines 661-832:**
   - Database tables: snake_case lowercase plural (projects, user_preferences, chat_messages)
   - Database columns: snake_case lowercase (project_id, created_at, token_count)
   - Rust functions/variables: snake_case (create_project, get_agent_status)
   - Rust types: PascalCase (Project, AgentExecution, Result<T, E>)
   - TypeScript functions/variables: camelCase (createProject, setActiveProject)
   - TypeScript types: PascalCase (Project, AppState)
   - File names: PascalCase components, snake_case utilities
   - Tauri commands: snake_case (create_project, generate_code)

5. **CLI as Library Pattern from Architecture.md lines 200-208:**
   - Commands MUST be library functions in demiarch-core/src/commands/
   - Both CLI and GUI share same commands via shared demiarch-core library
   - Commands defined with traits in interfaces/ for testability

**Security Constraints (MUST FOLLOW):**
1. **OS Keyring + Encrypted Fallback:**
   - Try OS keyring (keyring crate) FIRST
   - Fall back to encrypted SQLite table if OS keyring fails
   - Use zeroize to wipe plaintext from memory immediately after encryption
   - Store key in projects.openrouter_api_key_encrypted (encrypted column)

2. **API Key Encryption from Architecture.md lines 514-527:**
   - Use AES-GCM for encryption (aes-gcm crate)
   - Use argon2 for key derivation (argon2 crate) - CRITICAL REPLACEMENT for raw machine ID
   - Use ChaChaRng for nonce generation (rand_chacha crate)
   - Never reuse nonces

**Testing Constraints:**
- Unit tests co-located with modules
- Integration tests in tests/ directory at crate root
- Testing framework NOT yet defined - defer to Story 1.4
- Ensure all dependencies are compile-time checked via sqlx macros

### Library and Framework Requirements

**Rust Libraries:**
- tokio 1.41+ (async runtime) - Use [features = "full"] for async support
- sqlx 0.8+ (async database) - Use features = ["sqlite", "runtime-tokio"]
- rusqlite 0.32+ (SQLite extension loading) - Use [features = ["bundled"]
- serde 1.0 (serialization) - Use features = ["derive"]
- uuid 1.11 (unique IDs) - Use features = ["v4", "serde"]
- chrono 0.4 (time handling) - Use features = ["serde"]
- reqwest 0.12 (HTTP client) - Use features = ["json", "rustls-tls"]
- tauri 2.1 (Tauri framework)
- clap 4.5 (CLI parsing) - Use features = ["derive"]
- ratatui 0.29 (TUI library) - For CLI only
- thiserror 2.0 (error handling) - For all crates
- anyhow 1.0 (error handling) - For all crates

**Frontend Libraries:**
- react 18.3.0+ (UI framework)
- typescript 5.6.0+ (type safety)
- @tauri-apps/api 2.1.0+ (Tauri API bindings)

**Starter Template:**
- create-tauri-app@latest with --template react-ts flag
- npm package: create-tauri-app (NOT cargo command)

### File Structure Requirements

**MUST CREATE These Directories:**
1. **demiarch-core/src/domain/** with subdomains:**
   - projects/
   - agents/
   - context/
   - skills/
   - code_generation/
   - plugins/
   - cost/
   - security/
   - recovery/

2. **demiarch-core/src/application/****
   - Use case orchestrators (generate_code.rs, create_project.rs, manage_session.rs)

3. **demiarch-core/src/infrastructure/****
   - db/ (SQLite with sqlite-vec)
   - llm/ (OpenRouter client)
   - git/ (git2-rs integration)
   - wasm/ (Wasmtime runtime)

4. **demiarch-core/src/interfaces/****
   - Trait definitions for repository, storage, external services

5. **demiarch-cli/src/commands/****
   - CLI command implementations using shared command traits

**MUST CREATE These Entry Points:**
1. **demiarch-core/src/lib.rs** - Library exports with re-exports
2. **demiarch-cli/src/main.rs** - CLI entry point
3. **demiarch-gui/src-tauri/main.rs** - Tauri entry point using demiarch-core
4. **demiarch-gui/src/main.tsx** - React entry point

**Project Structure Notes:**

**Alignment with Unified Project Structure:**
- From Architecture.md lines 597-662: DDD structure strictly enforced
- All dependencies managed through workspace members
- Shared code in demiarch-core, specialized code in CLI/GUI crates
- CLI and GUI depend on demiarch-core via path-relative dependency: `demiarch-core = { path = "../demiarch-core" }`

**Detected Conflicts:**
None - This is the foundational story creating the project structure

**Rationale:**
Create-tauri-app provides basic Tauri structure but demiarch is a complex system requiring custom architecture from Day 1. This story establishes the workspace and foundational structure that all subsequent stories will build upon.

### Project Structure Requirements

**Cargo Workspace Structure:**
```
demiarch/
├── Cargo.toml                    # Workspace root with members
├── package.json                    # Optional workspace root
├── demiarch-core/                  # Shared business logic library
│   ├── Cargo.toml                 # Core dependencies
│   └── src/
│       ├── domain/               # Domain entities, value objects, repository traits
│       ├── application/          # Use cases/orchestrators
│       ├── infrastructure/       # SQLite, LLM client, git, WASM runtime
│       ├── interfaces/            # Trait definitions for external deps
│       └── lib.rs              # Library entry point
├── demiarch-cli/                   # CLI with ratatui
│   ├── Cargo.toml                 # CLI dependencies + shared lib
│   └── src/
│       ├── main.rs               # CLI entry point
│       ├── tui/                  # Terminal UI components
│       └── commands/              # CLI command handlers using shared lib
└── demiarch-gui/                   # Tauri + React GUI
    ├── Cargo.toml                 # Tauri dependencies + shared lib
    ├── src/                       # React frontend
    │   ├── main.tsx              # React entry point
    │   ├── components/            # shadcn/ui components
    │   ├── lib/                  # Shared utilities
    │   ├── hooks/                # React hooks
    │   └── styles/               # Tailwind CSS
    ├── src-tauri/                # Rust Tauri backend
    │   ├── main.rs               # Tauri entry point
    │   ├── commands/              # Tauri command implementations
    │   └── lib.rs                # Uses demiarch-core
    ├── package.json                # React dependencies
    ├── tsconfig.json              # TypeScript configuration
    ├── tailwind.config.js          # Tailwind configuration
    └── Tauri.toml                # Tauri configuration
```

**DDD Layer Responsibilities:**
- `domain/`: Business entities, value objects, domain services, repository traits
- `application/`: Use cases, orchestrators, command handlers
- `infrastructure/`: External integrations (SQLite, LLM client, Git, WASM)
- `interfaces/`: Trait definitions for cross-boundary communication

### Naming Conventions (MUST FOLLOW)

**Database:**
- Tables: snake_case, lowercase, plural nouns (e.g., `projects`, `user_preferences`, `chat_messages`)
- Columns: snake_case, lowercase (e.g., `project_id`, `created_at`, `token_count`)
- Foreign keys: `{table}_id` format (e.g., `project_id`, `agent_id`)
- Indexes: `idx_{table}_{column}` format (e.g., `idx_projects_name`, `idx_features_phase`)

**Rust Code:**
- Functions/variables: snake_case (e.g., `create_project`, `get_agent_status`)
- Types: PascalCase (e.g., `Project`, `AgentExecution`, `Result<T, E>`)
- File names: snake_case for modules, PascalCase for types
- Errors: Domain-specific enums in PascalCase (e.g., `ProjectError`, `AgentError`)

**TypeScript/React:**
- Components: PascalCase (e.g., `ProjectCard`, `AgentNode`)
- Functions/variables: camelCase (e.g., `createProject`, `setActiveProject`)
- Types: PascalCase (e.g., `Project`, `AppState`)
- File names: PascalCase for components, snake_case for utilities

**Tauri Commands:**
- snake_case (e.g., `create_project`, `generate_code`)
- Events: snake_case, past tense (e.g., `ProjectCreated`, `SessionStarted`)

**State Management:**
- Actions: camelCase, prefix with domain (e.g., `setActiveProject`, `clearAgents`)
- Selectors: camelCase with `select` prefix (e.g., `selectActiveProject`)

### Architecture Decisions

**Starter Template Selection:**
- Template: `create-tauri-app@latest demiarch --template react-ts`
- Rationale from Architecture.md lines 396-418:
  1. Foundation Alignment: Provides exact technology stack (Rust 1.41+, Tauri 2.x, React 18+, TypeScript 5.6+)
  2. Security Audited: Tauri undergoes security audits
  3. Cross-Platform: Native support for Linux, macOS, Windows
  4. Extensibility: Tauri plugin system for extensions
  5. Proven Stability: 102k+ GitHub stars, active maintenance

**Custom Architecture Required:**
From Architecture.md lines 406-417, create-tauri-app provides foundation but demiarch needs custom additions:
1. CLI crate with ratatui (separate or workspace member)
2. SQLite with vector extensions (sqlite-vec v0.1.6)
3. Agent orchestration framework (3-level Russian Doll hierarchy)
4. WASM plugin infrastructure with multi-tier verification
5. Progressive disclosure system with embedding support
6. Security layer (encryption, signing, keyring integration)
7. Cost management and budget enforcement
8. Recovery system with checkpointing
9. Conflict resolution with manual merge
10. Model routing with RL optimization

**Workspace Configuration Pattern:**
```toml
# demiarch/Cargo.toml (workspace root)
[workspace]
members = [
    "demiarch-core",
    "demiarch-cli", 
    "demiarch-gui",
]
resolver = "2"

[workspace.dependencies]
# Shared dependencies available to all workspace members
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.11", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "2.0"
```

### Testing Standards

**From Architecture.md lines 842-868:**
- Unit tests: Co-located with modules (`mod.rs` → `tests.rs` or `tests/` subdirectory)
- Integration tests: `tests/` directory at crate root
- E2E tests: Separate `e2e-tests/` directory (optional, for GUI testing)
- **Note:** Testing framework to be defined in Story 1.4 (add cargo test framework)

### Project Structure Notes

**Alignment with Architecture DDD:**
From Architecture.md lines 577-622, MUST follow DDD structure:
- `demiarch-core/src/domain/` contains:
  - `projects/` - Project entities, value objects, repository traits
  - `agents/` - Agent types, delegation logic
  - `context/` - Context retrieval, embeddings
  - `skills/` - Skill extraction, quality metrics
  - `code_generation/` - File operations, conflicts, rollback
  - `plugins/` - Plugin loading, sandboxing, licensing
  - `cost/` - Tracking, budgeting, alerts, model routing
  - `security/` - Encryption, signing, audit logging
  - `recovery/` - Checkpoints, transactions, cleanup

- `demiarch-core/src/application/` contains:
  - Orchestrators and use cases that coordinate domain logic

- `demiarch-core/src/infrastructure/` contains:
  - `db/` - SQLite with sqlite-vec
  - `llm/` - OpenRouter client
  - `git/` - git2-rs integration
  - `wasm/` - Wasmtime plugin runtime

**Detected Conflicts:**
None - This is the foundational story creating the project structure

**Rationale:**
Create-tauri-app provides basic Tauri structure but demiarch is a complex system requiring custom architecture from Day 1. This story establishes the workspace and foundational structure that all subsequent stories will build upon.

### References

- [PRD.md](docs/PRD.md) - Technology stack (lines 104-143), CLI commands (lines 422-493), Database schema (lines 230-894)
- [_bmad-output/planning-artifacts/architecture.md](_bmad-output/planning-artifacts/architecture.md) - Starter template selection (lines 396-418), tech stack (lines 96-104), project structure (lines 597-622), DDD structure (lines 577-622)
- [_bmad-output/planning-artifacts/epics.md](_bmad-output/planning-artifacts/epics.md) - Story 1.1 acceptance criteria (lines 162-175)
- [Architecture.md - Implementation Patterns](_bmad-output/planning-artifacts/architecture.md#Implementation-Patterns--Consistency-Rules) - Naming conventions (lines 661-710), DDD structure (lines 872-962)

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4 (20250514)

### Debug Log References

None

### Completion Notes List

- [x] Verified create-tauri-app command executed successfully
- [x] Workspace Cargo.toml created with 3 members
- [x] All three crate Cargo.toml files created
- [x] DDD directory structure established in demiarch-core
- [x] Core dependencies added to appropriate crates
- [x] Project builds successfully with `cargo build --workspace`
- [x] All crates pass `cargo check --workspace`
- [x] Success message displayed to user

### File List

**Files Created:**
- `demiarch/Cargo.toml` (workspace root)
- `demiarch-core/Cargo.toml`
- `demiarch-cli/Cargo.toml`
- `demiarch-gui/Cargo.toml`
- `demiarch-core/src/lib.rs` (library entry point)
- `demiarch-core/src/domain/mod.rs` (domain layer module)
- `demiarch-core/src/application/mod.rs` (application layer module)
- `demiarch-core/src/infrastructure/mod.rs` (infrastructure layer module)
- `demiarch-core/src/interfaces/mod.rs` (interfaces layer module)
- `demiarch-core/src/projects/mod.rs` (projects domain)
- `demiarch-core/src/projects/entity.rs` (Project entity)
- `demiarch-core/src/projects/value_object.rs` (Project value object)
- `demiarch-core/src/projects/repository.rs` (Project repository trait)
- `demiarch-core/src/projects/service.rs` (Project service)
- `demiarch-core/src/agents/mod.rs` (agents domain)
- `demiarch-core/src/agents/entity.rs` (Agent entity)
- `demiarch-core/src/agents/repository.rs` (Agent repository trait)
- `demiarch-core/src/agents/service.rs` (Agent service)
- `demiarch-core/src/domain/context/mod.rs` (context domain)
- `demiarch-core/src/domain/context/entity.rs` (Context entity)
- `demiarch-core/src/domain/context/repository.rs` (Context repository trait)
- `demiarch-core/src/domain/context/service.rs` (Context service)
- `demiarch-core/src/domain/skills/mod.rs` (skills domain)
- `demiarch-core/src/domain/skills/entity.rs` (Skills entity)
- `demiarch-core/src/domain/skills/repository.rs` (Skills repository trait)
- `demiarch-core/src/domain/skills/service.rs` (Skills service)
- `demiarch-core/src/domain/code_generation/mod.rs` (code_generation domain)
- `demiarch-core/src/domain/code_generation/entity.rs` (CodeGeneration entity)
- `demiarch-core/src/domain/code_generation/repository.rs` (CodeGeneration repository trait)
- `demiarch-core/src/domain/code_generation/service.rs` (CodeGeneration service)
- `demiarch-core/src/domain/plugins/mod.rs` (plugins domain)
- `demiarch-core/src/domain/plugins/entity.rs` (Plugins entity)
- `demiarch-core/src/domain/plugins/repository.rs` (Plugins repository trait)
- `demiarch-core/src/domain/plugins/service.rs` (Plugins service)
- `demiarch-core/src/domain/cost/mod.rs` (cost domain)
- `demiarch-core/src/domain/cost/entity.rs` (Cost entity)
- `demiarch-core/src/domain/cost/repository.rs` (Cost repository trait)
- `demiarch-core/src/domain/cost/service.rs` (Cost service)
- `demiarch-core/src/domain/security/mod.rs` (security domain)
- `demiarch-core/src/domain/security/entity.rs` (Security entity)
- `demiarch-core/src/domain/security/repository.rs` (Security repository trait)
- `demiarch-core/src/domain/security/service.rs` (Security service)
- `demiarch-core/src/domain/recovery/mod.rs` (recovery domain)
- `demiarch-core/src/domain/recovery/entity.rs` (Recovery entity)
- `demiarch-core/src/domain/recovery/repository.rs` (Recovery repository trait)
- `demiarch-core/src/domain/recovery/service.rs` (Recovery service)
- `demiarch-core/src/application/use_cases.rs` (use cases placeholder)
- `demiarch-core/src/application/services.rs` (application services placeholder)
- `demiarch-core/src/infrastructure/db.rs` (database infrastructure placeholder)
- `demiarch-core/src/infrastructure/llm.rs` (LLM infrastructure placeholder)
- `demiarch-core/src/infrastructure/git.rs` (Git infrastructure placeholder)
- `demiarch-core/src/infrastructure/wasm.rs` (WASM infrastructure placeholder)
- `demiarch-core/src/interfaces/repositories.rs` (repository interfaces placeholder)
- `demiarch-core/src/interfaces/services.rs` (service interfaces placeholder)
- `demiarch-core/src/interfaces/external.rs` (external service interfaces placeholder)
- `demiarch-cli/src/main.rs` (CLI entry point)
- `demiarch-gui/src-tauri/src/main.rs` (Tauri backend entry point)
- `demiarch-gui/src-tauri/src/lib.rs` (Tauri library entry point)
- `demiarch-gui/src/main.tsx` (React entry point)
- `demiarch-gui/src/App.tsx` (React App component)

**Files Modified (by create-tauri-app):**
- `demiarch-gui/package.json` (React dependencies)
- `demiarch-gui/tsconfig.json` (TypeScript configuration)
- `demiarch-gui/vite.config.ts` (Vite configuration)
- `demiarch-gui/index.html` (HTML entry point)
- `demiarch-gui/.gitignore` (Git ignore file)
- `demiarch-gui/public/` (static assets directory)

**Directories Created:**
- `demiarch-core/src/domain/{projects,agents,context,skills,code_generation,plugins,cost,security,recovery}/` (domain subdirectories)
- `demiarch-core/src/application/` (application layer)
- `demiarch-core/src/infrastructure/` (infrastructure layer)
- `demiarch-core/src/interfaces/` (interfaces layer)
- `demiarch-cli/src/` (CLI source)
- `demiarch-gui/src/` (React source)
- `demiarch-gui/src-tauri/` (Tauri backend)

**Configuration Files:**
- `demiarch/Cargo.toml` (workspace configuration)
- `demiarch/Cargo.lock` (dependency lock file)

**Note:** Subsequent stories (1.2-1.6) will build upon this foundational structure.

## Story Completion Status

**Status: ready-for-dev**

**Ultimate Context Engine Analysis Complete:**
- ✅ Exhaustive analysis of all artifacts (PRD, Architecture, Epics)
- ✅ Architecture compliance guardrails with precise version numbers and structure requirements
- ✅ Library and framework requirements with exact versions and features
- ✅ File structure requirements matching DDD specification
- ✅ Testing requirements and standards documented
- ✅ Project context with cross-references to source documents
- ✅ Dev Agent guardrails for flawless implementation

**Developer Has Everything Needed:**
- Complete technical stack with exact versions
- Exact directory structure to create (DDD layers)
- Precise naming conventions to follow (database, API, code)
- Workspace configuration pattern with resolver = "2"
- CLI as library pattern for shared command implementation
- Testing standards and approach
- Cross-document references for traceability
- No ambiguity - all decisions from architecture preserved

**Next Steps for Developer:**
1. Run create-tauri-app command with specified template
2. Verify workspace Cargo.toml is generated correctly
3. Create DDD directory structure in demiarch-core
4. Add dependencies to appropriate Cargo.toml files
5. Run `cargo check --workspace` to verify all crates compile
6. Run `cargo build --workspace` to verify successful build
7. Display success message to user

**Ready for dev-story execution!**
