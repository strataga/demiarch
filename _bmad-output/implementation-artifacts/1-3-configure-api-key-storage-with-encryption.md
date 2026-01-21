# Story 1.3: Configure API Key Storage with Encryption

Status: ready-for-dev

## Story

As a user,
I want to securely store my OpenRouter API key with encryption,
so that my credentials are protected and not exposed.

## Acceptance Criteria

1. **Given** User has an OpenRouter API key and Demiarch is running
   **When** User enters API key via CLI command `demiarch config set-api-key` or GUI settings panel
   **Then** System attempts to store key in OS keyring first (keyring crate)

2. **Given** OS keyring storage attempt
   **When** OS keyring fails or is unavailable
   **Then** Stores in encrypted SQLite table (projects.openrouter_api_key_encrypted)

3. **Given** API key being stored
   **When** Encryption is performed
   **Then** Key is encrypted using AES-GCM with argon2-derived key from machine ID

4. **Given** Encryption process
   **When** Nonce is needed
   **Then** Nonce is generated using ChaChaRng (cryptographically secure, never reused)

5. **Given** API key encryption complete
   **When** Memory cleanup occurs
   **Then** All plaintext containing key is zeroed from memory immediately after encryption (zeroize)

6. **Given** API key storage operation completes
   **When** User receives feedback
   **Then** User sees success message without key being displayed in logs or console

## Tasks / Subtasks

- [ ] Task 1: Implement OS keyring integration (AC: #1)
  - [ ] Add keyring crate dependency
  - [ ] Create KeyringStorage trait and implementation
  - [ ] Handle platform-specific keyring access (Linux, macOS, Windows)
  - [ ] Implement fallback detection when keyring unavailable

- [ ] Task 2: Implement AES-GCM encryption (AC: #2, #3)
  - [ ] Add aes-gcm, argon2 crate dependencies
  - [ ] Create machine ID derivation function
  - [ ] Implement argon2 key derivation from machine ID
  - [ ] Create AES-GCM encryption wrapper

- [ ] Task 3: Implement secure nonce generation (AC: #4)
  - [ ] Add rand_chacha crate dependency
  - [ ] Create cryptographically secure nonce generator using ChaChaRng
  - [ ] Ensure nonce uniqueness (never reused)

- [ ] Task 4: Implement memory hardening (AC: #5)
  - [ ] Add zeroize crate dependency
  - [ ] Apply Zeroize derive to all structs containing sensitive data
  - [ ] Ensure immediate zeroization after encryption

- [ ] Task 5: Implement CLI command (AC: #6)
  - [ ] Create `demiarch config set-api-key` command
  - [ ] Implement secure input (no echo to console)
  - [ ] Add success/failure feedback without exposing key

- [ ] Task 6: Implement SQLite encrypted storage (AC: #2)
  - [ ] Add encrypted_api_key column to schema
  - [ ] Create EncryptedKeyRepository implementation
  - [ ] Implement key retrieval with decryption

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Use keyring crate for OS keyring integration (preferred storage)
- Use aes-gcm 0.10 for encryption
- Use argon2 0.5 for key derivation
- Use rand_chacha for ChaChaRng nonce generation
- Use zeroize 1.8 for memory hardening

**Security Requirements:**
- API keys encrypted at rest using AES-GCM with machine-based key derivation (argon2)
- OS keyring storage preferred, encrypted SQLite fallback with zeroize
- Cryptographically secure nonce generation (ChaChaRng) - never reused
- Zero memory containing keys immediately after use

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/security/` - Security domain module
- `crates/demiarch-core/src/infrastructure/keyring/` - Keyring integration
- `crates/demiarch-core/src/infrastructure/crypto/` - Cryptographic operations
- `crates/demiarch-cli/src/commands/config.rs` - CLI config commands

**Naming Conventions:**
- Rust: snake_case for modules and functions
- Traits: PascalCase (e.g., `KeyStorage`, `Encryptor`)

### Testing Requirements

- Unit tests for encryption/decryption round-trip
- Unit tests for nonce uniqueness
- Integration tests for keyring fallback behavior
- Security tests for memory zeroization (verify with miri if possible)

### References

- [Source: docs/PRD.md#Security] - API key encryption requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#API-Key-Protection] - Detailed security architecture
- [Source: _bmad-output/planning-artifacts/epics.md#Story-1.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
