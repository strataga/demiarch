# Security Policy

## Overview

Demiarch takes a security-first approach to local-first AI app development. This document outlines security considerations, known issues, and best practices.

## Security Principles

1. **Local-First**: All data stays on your machine
2. **No Telemetry**: No data is sent to third-party servers
3. **Explicit Control**: No automatic operations (git, file writes, etc.)
4. **Sandboxed Plugins**: WASM plugins run in a constrained environment
5. **Environment Variables Only**: API keys and secrets are never stored in configuration files

## Vulnerability Reporting

If you discover a security vulnerability, please:

1. **DO NOT** open a public issue
2. Email security contact (to be added)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will:
- Acknowledge within 48 hours
- Provide regular updates
- Work with you to fix the issue
- Credit you in the fix

## Known Security Considerations

### Environment Variables

#### API Keys
- **Variable**: `DEMIARCH_API_KEY` or `OPENROUTER_API_KEY`
- **Purpose**: LLM API authentication
- **Storage**: Never stored in config files, only environment variables
- **Redaction**: Displayed in logs as `***XXXX` (last 4 chars only)

#### License Issuer Key
- **Variable**: `DEMIARCH_LICENSE_ISSUER_KEY`
- **Purpose**: Verify plugin licenses (Ed25519 public key, base64-encoded)
- **Required**: Yes, if license enforcement is enabled
- **Validation**: Key must be exactly 32 bytes

#### Unsafe Mode Flags

**⚠️ WARNING: These flags should NEVER be used in production**

##### `DEMIARCH_UNSAFE_ALLOW_UNLICENSED`
- **Purpose**: Disable plugin license enforcement
- **Risk Level**: HIGH
- **When Used**: Development and testing only
- **Dangers**:
  - Allows unverified plugins to execute
  - Bypasses cryptographic signature verification
  - Potential for malicious code execution
- **Safe Usage**:
  ```bash
  export DEMIARCH_REQUIRE_LICENSE=0
  export DEMIARCH_UNSAFE_ALLOW_UNLICENSED=1
  ```
  Only in:
  - Local development environments
  - Containerized test environments
  - CI/CD with isolated workspaces

##### `DEMIARCH_REQUIRE_LICENSE`
- **Purpose**: Enable/disable license enforcement
- **Default**: `true` (enforced)
- **Safe Setting**: `true` (always enforce)
- **Unsafe Setting**: `false` (requires `DEMIARCH_UNSAFE_ALLOW_UNLICENSED` to be set)

### Dependency Security

#### Acknowledged Advisories

The following security advisories are acknowledged but considered acceptable:

##### `paste` (v1.0.15) - Unmaintained
- **Advisory**: RUSTSEC-2024-0436
- **Risk Level**: Low
- **Location**: Transitive dependency via `ratatui` (TUI only)
- **Impact**: Affects terminal UI rendering only
- **Mitigation**: TUI is optional; core functionality unaffected
- **Status**: Monitoring for updates

##### `lru` (v0.12.5) - Unsound
- **Advisory**: RUSTSEC-2026-0002
- **Risk Level**: Low
- **Location**: Transitive dependency via `ratatui` (TUI only)
- **Impact**: Stacked Borrows violation in `IterMut` (TUI caching only)
- **Mitigation**: TUI is optional; core functionality unaffected
- **Status**: Monitoring for fix

### Plugin Security

#### WASM Sandbox Limits
All plugins execute in a constrained sandbox with the following limits:

```rust
Fuel limit:        10,000,000 operations
Memory limit:      16 MB
Table elements:    1,024
Instance limit:    16
Execution timeout: 5 seconds
```

#### Plugin Path Validation
- ✅ Symlinks are rejected
- ✅ Paths must reside within plugin directory
- ✅ Path traversal attacks are prevented
- ✅ File size limits enforced (64KB for manifests)

#### Permission System
Plugins must explicitly request permissions:
- All permissions must be granted before execution
- Duplicate permissions are rejected
- Imports are not allowed unless explicitly exposed
- Permissions are logged for auditing

### License Verification

#### Cryptographic Security
- **Algorithm**: Ed25519 (curve25519-dalek)
- **Hash**: SHA-256
- **Signature**: 64 bytes
- **Public Key**: 32 bytes
- **Key Source**: Environment variable (DEMIARCH_LICENSE_ISSUER_KEY)

#### Verification Process
1. License matches plugin ID
2. License has not expired
3. Public key matches trusted issuer
4. License payload matches manifest digest
5. Signature is cryptographically valid

### File System Security

#### Path Resolution
- All paths are canonicalized before use
- Symbolic links are rejected in plugin loading
- Paths validated against allowed directories
- No path traversal attacks possible

#### File Operations
- No unsafe file operations
- All file operations use Result types
- No use of `unwrap()` or `expect()` outside tests
- Proper error handling throughout

### Network Security

#### API Communication
- TLS 1.2+ required for all HTTP connections
- Certificate validation enforced
- No HTTP (unencrypted) connections
- reqwest with rustls-tls backend

#### Rate Limiting
- Not yet implemented (future feature)
- Will respect API provider rate limits
- Daily budget enforcement planned

### Code Security Practices

#### Rust Safety
- No `unsafe` blocks outside of necessary cryptographic operations
- No use of `unwrap()` or `expect()` in production code
- All errors handled via Result types
- Panic-free error handling

#### Input Validation
- User input validated before use
- No format string injection vulnerabilities
- No command injection vulnerabilities
- No SQL injection vulnerabilities (no SQL yet)

## Security Checklist

### Before Deployment
- [ ] `DEMIARCH_LICENSE_ISSUER_KEY` is set
- [ ] `DEMIARCH_UNSAFE_ALLOW_UNLICENSED` is NOT set
- [ ] `DEMIARCH_REQUIRE_LICENSE` is set to `true`
- [ ] API keys are in environment variables only
- [ ] No unverified plugins installed
- [ ] Security audit passes (`cargo audit`)
- [ ] Linting passes (`cargo clippy`)

### Development
- [ ] Run `cargo test --workspace`
- [ ] Run `cargo audit`
- [ ] Run `cargo clippy --workspace`
- [ ] Review all `unsafe` blocks
- [ ] Validate all error handling
- [ ] Check for new vulnerabilities

## Audit Results

### Last Audit: 2026-01-20
```
Dependencies scanned: 445
Vulnerabilities found: 0
Warnings: 2 (acknowledged, low risk)
Critical/High issues: 0
```

### Regular Audits
- `cargo audit` - Run weekly or on dependency updates
- `cargo clippy` - Run before every commit
- `cargo test` - Run before every commit
- Dependency updates - Review security advisories

## Threat Model

### Attacker Profiles

#### External Network Attackers
- **Access**: Limited to API endpoints (when implemented)
- **Impact**: Low - No public endpoints currently
- **Mitigation**: TLS, input validation, rate limiting (planned)

#### Malicious Plugin Authors
- **Access**: Plugin distribution
- **Impact**: Medium - Could execute malicious code if license verification bypassed
- **Mitigation**: Ed25519 signatures, sandbox limits, path validation

#### Local Users with Elevated Access
- **Access**: Direct file system, environment variables
- **Impact**: High - Could bypass all protections
- **Mitigation**: OS-level security (not in scope)

#### Compromised Dependency
- **Access**: Dependency supply chain
- **Impact**: High - Arbitrary code execution
- **Mitigation**: Regular audits, `deny.toml`, minimal dependencies

## Compliance

### Privacy
- GDPR compliant: All data local, no collection
- CCPA compliant: No data sharing or selling
- Industry best practices: Zero telemetry

### Licenses
- AGPL-3.0 for Demiarch
- Compatible with commercial use (see LICENSE)
- Plugin licensing: Ed25519-based verification

## Future Security Enhancements

### Planned
1. Rate limiting for API calls
2. Plugin reputation system
3. Automated dependency scanning in CI/CD
4. Security-focused CI/CD pipeline
5. Encryption at rest for sensitive data
6. Plugin sandboxing improvements

### Under Consideration
1. Hardware security module (HSM) support for license keys
2. Plugin code signing verification
3. Automated security testing
4. Fuzz testing for input handling
5. Formal verification for critical components

## Security Updates

Security updates will be:
- Documented in CHANGELOG.md
- Announced via repository
- Published as patch releases
- Backported to supported versions

## Contact

For security-related questions:
- Email: (to be added)
- GPG Key: (to be added)
- Bug Bounty: (to be added)

## Resources

- [Rust Security Guidelines](https://doc.rust-lang.org/nomicon/)
- [Cargo Audit](https://github.com/RustSec/cargo-audit)
- [Advisory Database](https://github.com/RustSec/advisory-db)
- [OWASP Rust Security](https://owasp.org/www-project-rust-security/)

---

**Last Updated**: 2026-01-20
**Version**: 0.1.0
