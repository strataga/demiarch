//! Ed25519 signing for checkpoint integrity verification
//!
//! Provides cryptographic signing and verification for checkpoints to ensure
//! data integrity and detect tampering.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_chacha::rand_core::{OsRng, RngCore};
use thiserror::Error;

/// Size of Ed25519 signature in bytes
pub const SIGNATURE_SIZE: usize = 64;

/// Size of Ed25519 public key in bytes
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Size of Ed25519 private key in bytes
pub const PRIVATE_KEY_SIZE: usize = 32;

/// Errors that can occur during signing operations
#[derive(Debug, Error)]
pub enum SigningError {
    #[error("Signature verification failed")]
    VerificationFailed,

    #[error("Invalid signature length: expected {expected}, got {actual}")]
    InvalidSignatureLength { expected: usize, actual: usize },

    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("Signing failed: {0}")]
    SigningFailed(String),
}

/// Ed25519 signing key pair for checkpoint integrity
///
/// The signing key (private key) is used to sign checkpoint data,
/// and the verifying key (public key) is compiled into the binary
/// for verification.
#[derive(Debug)]
pub struct CheckpointSigner {
    signing_key: SigningKey,
}

impl CheckpointSigner {
    /// Generate a new random signing key pair
    pub fn generate() -> Self {
        let mut secret_bytes = [0u8; PRIVATE_KEY_SIZE];
        OsRng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        Self { signing_key }
    }

    /// Create a signer from raw private key bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        if bytes.len() != PRIVATE_KEY_SIZE {
            return Err(SigningError::InvalidKeyLength {
                expected: PRIVATE_KEY_SIZE,
                actual: bytes.len(),
            });
        }

        let mut key_bytes = [0u8; PRIVATE_KEY_SIZE];
        key_bytes.copy_from_slice(bytes);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        Ok(Self { signing_key })
    }

    /// Export the private key bytes (use carefully!)
    pub fn to_bytes(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.signing_key.to_bytes()
    }

    /// Get the public verifying key bytes
    pub fn verifying_key_bytes(&self) -> [u8; PUBLIC_KEY_SIZE] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Sign checkpoint data
    ///
    /// The data should be the serialized snapshot JSON.
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(data);
        signature.to_bytes().to_vec()
    }

    /// Verify a signature against data
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), SigningError> {
        if signature.len() != SIGNATURE_SIZE {
            return Err(SigningError::InvalidSignatureLength {
                expected: SIGNATURE_SIZE,
                actual: signature.len(),
            });
        }

        let sig_bytes: [u8; SIGNATURE_SIZE] = signature
            .try_into()
            .map_err(|_| SigningError::VerificationFailed)?;

        let signature = Signature::from_bytes(&sig_bytes);
        self.signing_key
            .verifying_key()
            .verify(data, &signature)
            .map_err(|_| SigningError::VerificationFailed)
    }
}

/// Verifier for checkpoint signatures using a public key
///
/// This can be used to verify signatures without having the private key,
/// using only the public key that's compiled into the binary.
#[derive(Debug, Clone)]
pub struct CheckpointVerifier {
    verifying_key: VerifyingKey,
}

impl CheckpointVerifier {
    /// Create a verifier from raw public key bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        if bytes.len() != PUBLIC_KEY_SIZE {
            return Err(SigningError::InvalidKeyLength {
                expected: PUBLIC_KEY_SIZE,
                actual: bytes.len(),
            });
        }

        let mut key_bytes = [0u8; PUBLIC_KEY_SIZE];
        key_bytes.copy_from_slice(bytes);

        let verifying_key =
            VerifyingKey::from_bytes(&key_bytes).map_err(|_| SigningError::VerificationFailed)?;

        Ok(Self { verifying_key })
    }

    /// Verify a signature against data
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), SigningError> {
        if signature.len() != SIGNATURE_SIZE {
            return Err(SigningError::InvalidSignatureLength {
                expected: SIGNATURE_SIZE,
                actual: signature.len(),
            });
        }

        let sig_bytes: [u8; SIGNATURE_SIZE] = signature
            .try_into()
            .map_err(|_| SigningError::VerificationFailed)?;

        let signature = Signature::from_bytes(&sig_bytes);
        self.verifying_key
            .verify(data, &signature)
            .map_err(|_| SigningError::VerificationFailed)
    }
}

/// Default checkpoint signing key for local development
///
/// In production, this would be generated once and the public key
/// compiled into the binary while the private key is kept secure.
/// For local-first operation, we generate a key per installation
/// and store it securely (e.g., in the keyring or encrypted config).
///
/// This function returns a deterministic key for testing purposes only.
/// Real implementations should use `CheckpointSigner::generate()`.
#[cfg(test)]
pub fn test_signing_key() -> CheckpointSigner {
    // Fixed test key for reproducible tests
    let bytes: [u8; 32] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    CheckpointSigner::from_bytes(&bytes).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_signer() {
        let signer = CheckpointSigner::generate();
        let verifying_key = signer.verifying_key_bytes();
        assert_eq!(verifying_key.len(), PUBLIC_KEY_SIZE);
    }

    #[test]
    fn test_sign_and_verify() {
        let signer = CheckpointSigner::generate();
        let data = b"Hello, checkpoint!";

        let signature = signer.sign(data);
        assert_eq!(signature.len(), SIGNATURE_SIZE);

        // Verification should succeed
        assert!(signer.verify(data, &signature).is_ok());
    }

    #[test]
    fn test_verify_fails_with_wrong_data() {
        let signer = CheckpointSigner::generate();
        let data = b"Original data";
        let signature = signer.sign(data);

        // Verification should fail with different data
        let result = signer.verify(b"Modified data", &signature);
        assert!(matches!(result, Err(SigningError::VerificationFailed)));
    }

    #[test]
    fn test_verify_fails_with_wrong_signature() {
        let signer = CheckpointSigner::generate();
        let data = b"Some data";

        // Create a wrong signature
        let wrong_signature = vec![0u8; SIGNATURE_SIZE];

        let result = signer.verify(data, &wrong_signature);
        assert!(matches!(result, Err(SigningError::VerificationFailed)));
    }

    #[test]
    fn test_verifier_from_public_key() {
        let signer = CheckpointSigner::generate();
        let public_key = signer.verifying_key_bytes();
        let data = b"Test data";

        // Sign with the signer
        let signature = signer.sign(data);

        // Verify with just the public key
        let verifier = CheckpointVerifier::from_bytes(&public_key).unwrap();
        assert!(verifier.verify(data, &signature).is_ok());
    }

    #[test]
    fn test_signer_roundtrip() {
        let signer1 = CheckpointSigner::generate();
        let bytes = signer1.to_bytes();

        let signer2 = CheckpointSigner::from_bytes(&bytes).unwrap();
        assert_eq!(
            signer1.verifying_key_bytes(),
            signer2.verifying_key_bytes()
        );

        // Both should produce same signature
        let data = b"Test data";
        let sig1 = signer1.sign(data);
        let sig2 = signer2.sign(data);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_invalid_signature_length() {
        let signer = CheckpointSigner::generate();
        let data = b"Test";
        let short_sig = vec![0u8; 32]; // Too short

        let result = signer.verify(data, &short_sig);
        assert!(matches!(
            result,
            Err(SigningError::InvalidSignatureLength { .. })
        ));
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = vec![0u8; 16]; // Too short
        let result = CheckpointSigner::from_bytes(&short_key);
        assert!(matches!(result, Err(SigningError::InvalidKeyLength { .. })));
    }

    #[test]
    fn test_json_checkpoint_signing() {
        let signer = CheckpointSigner::generate();

        // Simulate signing a JSON checkpoint
        let snapshot = serde_json::json!({
            "phases": [{"id": "1", "name": "Planning"}],
            "features": [{"id": "2", "title": "Auth"}],
            "chat_messages": [],
            "generated_code": []
        });

        let data = serde_json::to_vec(&snapshot).unwrap();
        let signature = signer.sign(&data);

        assert!(signer.verify(&data, &signature).is_ok());

        // Tampered data should fail
        let tampered = serde_json::json!({
            "phases": [{"id": "1", "name": "TAMPERED"}],
            "features": [{"id": "2", "title": "Auth"}],
            "chat_messages": [],
            "generated_code": []
        });
        let tampered_data = serde_json::to_vec(&tampered).unwrap();
        assert!(signer.verify(&tampered_data, &signature).is_err());
    }
}
