//! Offline license verification using ed25519 signatures

use crate::{PluginError, PluginManifest, PluginResult};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::env;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct License {
    pub plugin_id: String,
    pub expires_at: DateTime<Utc>,
    /// 32-byte public key (must match trusted issuer key)
    pub public_key: Vec<u8>,
    /// 64-byte signature over payload
    pub signature: Vec<u8>,
    /// Arbitrary payload that was signed (e.g., manifest digest)
    pub payload: Vec<u8>,
}

/// Verify a plugin license (expiry + signature)
pub fn verify_license(manifest: &PluginManifest, license: &License) -> PluginResult<()> {
    if manifest.id != license.plugin_id {
        return Err(PluginError::ValidationFailed(
            "License plugin id does not match manifest".to_string(),
        ));
    }

    if Utc::now() > license.expires_at {
        return Err(PluginError::LicenseExpired(manifest.id.clone()));
    }

    if license.public_key.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
        return Err(PluginError::ValidationFailed(
            "License public key must be 32 bytes".to_string(),
        ));
    }

    let issuer_key = resolve_issuer_key()?;

    if license.public_key.as_slice() != issuer_key.as_bytes() {
        return Err(PluginError::ValidationFailed(
            "License issuer key is not trusted".to_string(),
        ));
    }

    let expected_payload = manifest_digest(manifest)?;
    if license.payload != expected_payload {
        return Err(PluginError::ValidationFailed(
            "License payload does not match manifest".to_string(),
        ));
    }

    let sig = Signature::from_slice(&license.signature)
        .map_err(|e| PluginError::ValidationFailed(format!("Invalid signature: {e}")))?;

    issuer_key
        .verify(&license.payload, &sig)
        .map_err(|e| PluginError::ValidationFailed(format!("License verification failed: {e}")))
}

fn resolve_issuer_key() -> PluginResult<VerifyingKey> {
    let key_b64 = env::var("DEMIARCH_LICENSE_ISSUER_KEY").map_err(|_| {
        PluginError::ValidationFailed(
            "Missing trusted issuer key (set DEMIARCH_LICENSE_ISSUER_KEY)".to_string(),
        )
    })?;

    let key_bytes = BASE64_STANDARD
        .decode(key_b64)
        .map_err(|e| PluginError::ValidationFailed(format!("Invalid issuer key encoding: {e}")))?;

    if key_bytes.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
        return Err(PluginError::ValidationFailed(
            "Issuer key must be 32 bytes".to_string(),
        ));
    }

    let key_array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = key_bytes
        .try_into()
        .map_err(|_| PluginError::ValidationFailed("Issuer key must be 32 bytes".to_string()))?;

    VerifyingKey::from_bytes(&key_array)
        .map_err(|e| PluginError::ValidationFailed(format!("Invalid issuer key: {e}")))
}

fn manifest_digest(manifest: &PluginManifest) -> PluginResult<Vec<u8>> {
    let serialized = serde_json::to_vec(manifest).map_err(|e| {
        PluginError::ValidationFailed(format!("Failed to serialize manifest for digest: {e}"))
    })?;

    let mut hasher = Sha256::new();
    hasher.update(serialized);
    Ok(hasher.finalize().to_vec())
}
