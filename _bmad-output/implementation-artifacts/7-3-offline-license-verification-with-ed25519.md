# Story 7.3: Offline License Verification with ed25519

Status: ready-for-dev

## Story

As a user,
I want Demiarch to verify plugin licenses offline using ed25519 signatures,
so that I can use plugins without internet connectivity.

## Acceptance Criteria

1. **Given** User installs a plugin with licensing
   **When** Plugin is installed
   **Then** plugin_licenses record is created with plugin_id, license_key, license_type, tier, issued_at, expires_at

2. **Given** License is stored
   **When** Signature is verified
   **Then** Signature is verified using public key compiled into binary (const array)

3. **Given** Signature is invalid
   **When** Validation fails
   **Then** validation_status is set to 'invalid' and user sees error

4. **Given** Signature is valid
   **When** Validation succeeds
   **Then** validation_status is set to 'valid', validated_at is set to current time

5. **Given** License has expired
   **When** Expiry is checked
   **Then** validation_status is 'expired' and user sees: "License expired on {expires_at}"

6. **Given** License is valid
   **When** Features are enabled
   **Then** Plugin functionality is available based on tier (free, paid features enabled)

## Tasks / Subtasks

- [ ] Task 1: Create plugin_licenses table schema (AC: #1)
  - [ ] Add migration for plugin_licenses table
  - [ ] Include columns: id, plugin_id, license_key, license_type, tier, issued_at, expires_at, validation_status, validated_at
  - [ ] Add foreign key to installed_plugins

- [ ] Task 2: Embed public keys in binary (AC: #2)
  - [ ] Generate ed25519 key pair for Demiarch
  - [ ] Embed public key as const array
  - [ ] Document key rotation process

- [ ] Task 3: Implement license verification (AC: #2, #3, #4)
  - [ ] Parse license_key format
  - [ ] Extract signature and payload
  - [ ] Verify signature with ed25519-dalek
  - [ ] Update validation_status

- [ ] Task 4: Implement expiry checking (AC: #5)
  - [ ] Check expires_at against current time
  - [ ] Set status to 'expired' if past
  - [ ] Display expiry message to user

- [ ] Task 5: Implement tier-based features (AC: #6)
  - [ ] Define feature tiers (free, basic, pro)
  - [ ] Enable/disable features based on tier
  - [ ] Show upgrade prompts for locked features

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Offline license verification with ed25519 signatures
- Public key embedded in binary
- Tier-based feature gating

**Security Requirements:**
- Private key never stored in binary
- Signature verification before feature access
- Clear messaging for expired licenses

### License Format

```rust
// License key format: base64(payload || signature)
// Payload: JSON { plugin_id, tier, issued_at, expires_at }
// Signature: ed25519 signature of payload

pub struct LicensePayload {
    pub plugin_id: String,
    pub tier: LicenseTier,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub enum LicenseTier {
    Free,
    Basic,
    Pro,
}

pub enum ValidationStatus {
    Valid,
    Invalid,
    Expired,
    Pending,
}

// Embedded public key (example - replace with real key)
pub const DEMIARCH_PUBLIC_KEY: [u8; 32] = [
    // ed25519 public key bytes
    0x00, 0x01, 0x02, /* ... */
];

pub fn verify_license(license_key: &str) -> Result<LicensePayload> {
    let decoded = base64::decode(license_key)?;
    let (payload_bytes, signature_bytes) = decoded.split_at(decoded.len() - 64);

    let public_key = PublicKey::from_bytes(&DEMIARCH_PUBLIC_KEY)?;
    let signature = Signature::from_bytes(signature_bytes)?;

    public_key.verify(payload_bytes, &signature)?;

    let payload: LicensePayload = serde_json::from_slice(payload_bytes)?;

    // Check expiry
    if payload.expires_at < Utc::now() {
        return Err(LicenseError::Expired(payload.expires_at));
    }

    Ok(payload)
}
```

### Project Structure Notes

**File Locations:**
- `crates/demiarch-core/src/domain/plugins/licensing/license.rs`
- `crates/demiarch-core/src/domain/plugins/licensing/verifier.rs`
- `crates/demiarch-core/src/domain/plugins/licensing/keys.rs`
- `crates/demiarch-core/src/infrastructure/db/plugin_licenses.rs`

**Database Schema:**
```sql
CREATE TABLE plugin_licenses (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    license_key TEXT NOT NULL,
    license_type TEXT NOT NULL,  -- 'perpetual', 'subscription', 'trial'
    tier TEXT NOT NULL,  -- 'free', 'basic', 'pro'
    issued_at DATETIME NOT NULL,
    expires_at DATETIME,
    validation_status TEXT DEFAULT 'pending',
    validated_at DATETIME,
    FOREIGN KEY (plugin_id) REFERENCES installed_plugins(id)
);

CREATE UNIQUE INDEX idx_plugin_licenses_plugin ON plugin_licenses(plugin_id);
```

### Testing Requirements

- Signature verification tests
- Invalid signature rejection tests
- Expiry checking tests
- Tier feature gating tests

### References

- [Source: docs/PRD.md#Plugin-System] - License requirements
- [Source: _bmad-output/planning-artifacts/architecture.md#Plugin-System] - ed25519 details
- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.3] - Original story definition

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
