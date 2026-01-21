# Changelog

All notable changes to Demiarch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-20

### Summary
Initial release of Demiarch with core infrastructure, security framework, and comprehensive documentation.

---

### 2026-01-20

#### Security
- Implemented plugin dependencies and license verification
- Zero critical or high vulnerabilities (cargo audit)
- License key validation at startup with Ed25519 signature verification
- Plugin license verification system implemented
- SECURITY.md with comprehensive security policy
- API keys only via environment variables (never stored)
- Path traversal protection for plugin loading
- Symlink rejection in plugin validation
- File size limits for plugin manifests (64KB)
- WASM sandbox limits: 10M fuel, 16MB memory, 5s timeout
- Security advisories acknowledged and documented (paste, lru crates)

#### Infrastructure
- Synced workspace changes across all crates
- Fixed WSL target path in gitignore

#### Configuration
- Repository URL updated: https://github.com/demiarch/demiarch
- Updated deny.toml to acknowledge RUSTSEC-2026-0002 (lru crate)

---

### 2026-01-19

#### Core Features
- Database infrastructure layer added (domain-driven design)
- Added comprehensive error and config tests (66 tests)
- Agent type enumeration: Orchestrator, Planner, Coder, Reviewer, Tester
- Command interfaces: project, feature, generate, sync, chat
- Configuration management with environment variable support

#### Testing
- 71 unit tests covering core functionality
- Test coverage: ~35% (placeholder modules reduce overall percentage)
- Test modules: commands (20), error (40), config (6)

#### Documentation
- TEST_COVERAGE_REPORT.md with detailed coverage analysis
- Code review findings for SQLite setup story
- README with work-in-progress notice

#### Infrastructure
- Added Cargo.lock files for reproducible builds
- Updated gitignore to track Cargo.lock and VSCode extensions
- VS Code recommended extensions (.vscode/extensions.json)
- Created rust.yml GitHub Actions workflow

#### GUI Components
- Added Node.js TypeScript configuration for Tauri GUI
- Created Tauri GUI dependency installation script (install-gui-deps.sh)

---

### 2026-01-18

#### Initial Setup
- Initial commit: Demiarch project scaffold
- Created rust.yml GitHub Actions workflow
- Added work-in-progress notice to README

---

## Version 0.1.0 - Complete Feature List

### Security
- ✅ Zero critical or high vulnerabilities
- ✅ License key validation at startup
- ✅ Plugin license verification (Ed25519)
- ✅ API keys via environment variables only
- ✅ Path traversal protection
- ✅ Symlink rejection
- ✅ WASM sandbox with limits
- ✅ Security audit integration (CI/CD)

### Core Library (demiarch-core)
- ✅ Error types with 23 variants and helpful suggestions
- ✅ Command interfaces (project, feature, generate, sync, chat)
- ✅ Configuration management
- ✅ Agent system (type definitions)
- ✅ Plugin system foundation
- ⚠️ Database infrastructure (domain layer only)
- ⏳ Cost management (placeholder)
- ⏳ Context management (placeholder)
- ⏳ Skills system (placeholder)
- ⏳ Hooks system (placeholder)
- ⏳ LLM integration (placeholder)
- ⏳ Model routing (placeholder)

### CLI Binary (demiarch-cli)
- ✅ Clap-based argument parsing
- ✅ Doctor command (health check)
- ✅ Structured logging with tracing
- ✅ License key validation
- ✅ Error handling (no panics)
- ✅ Framework for 14 total commands
- ⚠️ Only doctor implemented (1/14)

### TUI Binary (demiarch-tui)
- ✅ Basic project structure
- ✅ Ratatui framework configured
- ⏳ Implementation pending (placeholder)

### Plugin System (demiarch-plugins)
- ✅ WASM sandbox execution via wasmtime
- ✅ Plugin manifest validation
- ✅ License verification
- ✅ Permission-based access control
- ✅ Path validation
- ✅ Symlink rejection
- ✅ File size limits

### Testing
- ✅ 71 unit tests (100% pass rate)
- ✅ 0 integration tests (framework in place)

### Documentation
- ✅ README.md with project overview
- ✅ CONTRIBUTING.md with contribution guidelines
- ✅ SECURITY.md with security policy
- ✅ AUTHORS.md for contributor recognition
- ✅ CHANGELOG.md with version history
- ✅ TEST_COVERAGE_REPORT.md with coverage analysis
- ✅ ADDITIONAL_ISSUES.md with issue documentation
- ✅ SECURITY_FIXES.md with fix documentation

### CI/CD
- ✅ GitHub Actions workflow for:
  - Automated linting (fmt, clippy)
  - Automated testing
  - Security audits (deny, audit)
  - Dependency caching
  - Multi-workspace support

### Configuration
- ✅ deny.toml with dependency security rules
- ✅ .gitignore with standard exclusions
- ✅ Cargo.lock for reproducible builds
- ✅ VS Code extensions configuration
- ✅ TypeScript configuration for GUI

---

## Version History

| Version | Date | Status | Notes |
|---------|------|--------|-------|
| 0.1.0 | 2026-01-20 | ✅ Released | Initial release with core infrastructure |
| Unreleased | TBD | 🚧 In Development | Issue fixes and improvements |

---

## Commits Summary

### Total Commits: 15
- 2026-01-20: 3 commits
- 2026-01-19: 9 commits
- 2026-01-18: 3 commits

### Commit Types
- **feat**: 1 (database infrastructure)
- **docs**: 2 (code review, test coverage)
- **test**: 1 (comprehensive tests)
- **chore**: 11 (configuration, gitignore, dependencies, etc.)

---

**Note**: This changelog is maintained to reflect actual git commit history. Update it when making user-facing changes.

For contributor guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).
For security information, see [SECURITY.md](SECURITY.md).
For detailed analysis of issues, see [ADDITIONAL_ISSUES.md](ADDITIONAL_ISSUES.md).
