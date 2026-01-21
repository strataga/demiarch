# Contributing to Demiarch

Thank you for your interest in contributing to Demiarch! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)

## Code of Conduct

### Our Pledge

In the interest of fostering an open and welcoming environment, we as contributors and maintainers pledge to make participation in our project and our community a harassment-free experience for everyone.

### Our Standards

Examples of behavior that contributes to a positive environment include:
- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

Examples of unacceptable behavior include:
- The use of sexualized language or imagery
- Trolling, insulting/derogatory comments, or personal/political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other unethical or unprofessional conduct

## Getting Started

### Prerequisites

- **Rust**: 1.85 or later
- **Cargo**: Included with Rust
- **Git**: For version control

### Setting Up Development Environment

1. **Clone the repository**:
   ```bash
   git clone https://github.com/demiarch/demiarch.git
   cd demiarch
   ```

2. **Install development tools**:
   ```bash
   # Install cargo-deny for dependency security checks
   cargo install cargo-deny

   # Install cargo-audit for vulnerability scanning
   cargo install cargo-audit
   ```

3. **Build the project**:
   ```bash
   cargo build --workspace
   ```

4. **Run tests**:
   ```bash
   cargo test --workspace
   ```

5. **Run the CLI**:
   ```bash
   cargo run -p demiarch-cli -- --help
   ```

### Project Structure

```
demiarch/
├── crates/
│   ├── demiarch-core/      # Core library
│   ├── demiarch-cli/       # CLI binary
│   ├── demiarch-tui/       # TUI binary (optional)
│   └── demiarch-plugins/   # Plugin system
├── gui/                   # Tauri GUI (future)
├── docs/                  # Documentation
├── migrations/             # Database migrations
└── tests/                 # Integration tests
```

## Development Workflow

### Branch Naming

Use descriptive branch names:
- `feat/add-feature-name` - New features
- `fix/fix-bug-description` - Bug fixes
- `docs/update-documentation` - Documentation changes
- `refactor/refactor-description` - Code refactoring
- `test/add-test-coverage` - Test additions

### Making Changes

1. **Create a new branch**:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make your changes** following the [Coding Standards](#coding-standards)

3. **Test your changes**:
   ```bash
   # Run all tests
   cargo test --workspace

   # Run specific test
   cargo test --package demiarch-core test_name

   # Run clippy
   cargo clippy --workspace

   # Run security audit
   cargo audit
   ```

4. **Format your code**:
   ```bash
   cargo fmt --all
   ```

5. **Check documentation**:
   ```bash
   cargo doc --no-deps --open
   ```

6. **Commit your changes** following [Commit Guidelines](#commit-guidelines)

## Coding Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for code formatting
- Follow the standard naming conventions:
  - Types: `PascalCase`
  - Functions: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`
  - Modules: `snake_case`

### Documentation

- **All public items must have documentation**:
  ```rust
  //! Module-level documentation

  /// Brief description
  ///
  /// Longer description if needed
  ///
  /// # Examples
  ///
  /// ```rust
  /// let result = function();
  /// assert_eq!(result, expected);
  /// ```
  pub fn function() -> Result<()> {
      // ...
  }
  ```

- **Example documentation template**:
  ```rust
  //! Cost management and budget enforcement
  //!
  //! This module provides cost tracking and budget management for LLM API calls.
  //!
  //! # Features
  //!
  //! - Daily budget limits
  //! - Per-project cost tracking
  //! - Real-time cost monitoring
  //!
  //! # Usage
  //!
  //! ```rust
  //! use demiarch_core::cost::CostManager;
  //!
  //! let manager = CostManager::new();
  //! manager.track_cost(0.05, "project-id").await?;
  //! ```
  ```

### Error Handling

- **Use the Error type from demiarch_core**:
  ```rust
  use demiarch_core::{Error, Result};

  pub fn function() -> Result<String> {
      Ok("value".to_string())
  }
  ```

- **Never use `unwrap()` or `expect()` in production code**:
  ```rust
  // ❌ Bad
  let value = option.unwrap();

  // ✅ Good
  let value = option.ok_or_else(|| Error::InvalidInput("Missing value".to_string()))?;
  ```

- **Use `?` operator for error propagation**:
  ```rust
  pub async fn do_work() -> Result<()> {
      step1()?;
      step2()?;
      Ok(())
  }
  ```

### Logging

- **Use `tracing` for logging**:
  ```rust
  use tracing::{info, warn, error, debug};

  info!("Starting operation");
  debug!("Debug details: {:?}", data);
  warn!("Warning condition");
  error!("Error occurred: {}", err);
  ```

- **Use `println!` only for user-facing CLI output**:
  ```rust
  // For CLI output (user-facing)
  println!("Operation completed successfully");

  // For logging (developer-facing)
  info!("Operation completed successfully");
  ```

### Security

- **Never hardcode secrets**:
  ```rust
  // ❌ Bad
  let api_key = "sk-or-...";

  // ✅ Good
  let api_key = std::env::var("DEMIARCH_API_KEY")?;
  ```

- **Validate all inputs**:
  ```rust
  pub fn create_project(name: &str) -> Result<()> {
      if name.trim().is_empty() {
          return Err(Error::InvalidInput("Name cannot be empty".to_string()));
      }
      // ...
  }
  ```

- **Use Result types, never panic in production code**

## Testing Requirements

### Unit Tests

- **Write tests for all public functions**:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[tokio::test]
      async fn test_function_success() {
          let result = function().await;
          assert!(result.is_ok());
      }

      #[tokio::test]
      async fn test_function_error_case() {
          let result = function().await;
          assert!(result.is_err());
      }
  }
  ```

- **Test both success and error cases**

- **Use descriptive test names**

### Integration Tests

- **Add integration tests for workflows**:
  ```rust
  // crates/demiarch-core/tests/integration_tests.rs

  #[tokio::test]
  async fn test_create_project_workflow() {
      let project_id = project::create("test", "nextjs", "https://example.com").await.unwrap();
      let projects = project::list().await.unwrap();
      assert!(projects.contains(&project_id));
      project::delete(&project_id, true).await.unwrap();
  }
  ```

### Test Coverage

- **Aim for 70%+ coverage** on new code
- **Run tests before committing**: `cargo test --workspace`
- **Check coverage**: `cargo tarpaulin --out Html` (optional)

## Commit Guidelines

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Code style changes (formatting, etc.)
- **refactor**: Code refactoring
- **test**: Adding or updating tests
- **chore**: Maintenance tasks
- **security**: Security fixes

### Examples

```
feat(cli): add project list command

Implemented the project list command with filtering options.

- Added list() function to project module
- Added filtering by status
- Added pagination support

Closes #123
```

```
fix(core): resolve panic on empty input

Added validation to prevent panics when user provides empty project name.

Fixes #456
```

### Commit Checklist

Before committing, ensure:
- [ ] All tests pass: `cargo test --workspace`
- [ ] Code is formatted: `cargo fmt --all`
- [ ] No clippy warnings: `cargo clippy --workspace`
- [ ] No security issues: `cargo audit`
- [ ] Documentation is updated
- [ ] Commit message follows guidelines

## Pull Request Process

### Before Submitting

1. **Update documentation** if needed
2. **Add tests** for new functionality
3. **Update CHANGELOG.md** (if user-facing changes)
4. **Run full test suite**:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo audit
   ```

### PR Description Template

Use this template for PR descriptions:

```markdown
## Description
Brief description of what this PR does.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Refactoring

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests pass locally
- [ ] Manual testing performed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Commented complex code sections
- [ ] Documentation updated
- [ ] No new warnings generated
- [ ] Tests added for new functionality
- [ ] CHANGELOG.md updated (if applicable)

## Related Issues
Closes #123
Related to #456
```

### PR Review Process

1. **Automated checks**:
   - CI runs tests
   - CI runs linting
   - CI runs security audit

2. **Manual review**:
   - Code review by maintainers
   - Request changes if needed
   - Approve when ready

3. **Merge**:
   - Squash and merge to main
   - Delete feature branch
   - Update CHANGELOG.md if needed

### Getting Feedback

- Be patient with the review process
- Respond to review comments promptly
- Make requested changes or discuss alternatives
- Ask questions if anything is unclear

## Reporting Bugs

### Before Reporting

1. **Check existing issues** to avoid duplicates
2. **Search closed issues** to see if already fixed
3. **Verify you're on the latest version**

### Bug Report Template

```markdown
## Bug Description
A clear and concise description of what the bug is.

## Reproduction Steps
1. Step 1
2. Step 2
3. ...

## Expected Behavior
What you expected to happen.

## Actual Behavior
What actually happened.

## Environment
- OS: [e.g., Ubuntu 22.04, macOS 13, Windows 11]
- Demiarch version: [e.g., 0.1.0]
- Rust version: [e.g., 1.85]

## Logs/Error Messages
```
Paste any relevant logs or error messages here
```

## Additional Context
Any other context, screenshots, or examples.
```

## Suggesting Features

### Feature Request Template

```markdown
## Feature Description
A clear and concise description of the feature.

## Problem Statement
What problem does this feature solve? What use cases does it address?

## Proposed Solution
How should this feature work?

## Alternatives Considered
What other approaches did you consider?

## Additional Context
Any other context, mockups, or examples.
```

## Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and discussions
- **Documentation**: Check existing docs first
- **Security Issues**: See [SECURITY.md](SECURITY.md)

## Recognition

Contributors will be recognized in:
- `AUTHORS.md` file
- Release notes
- Contributor list in README

Thank you for contributing to Demiarch!
