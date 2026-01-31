//! Cryptographic primitives for CHINJU Protocol

use crate::types::{Hash, HashAlgorithm, Signature, SignatureAlgorithm, Timestamp};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use sha3::{Digest, Sha3_256, Sha3_512};
use thiserror::Error;

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Signature verification failed")]
    VerificationFailed,
    #[error("Unsupported algorithm: {0:?}")]
    UnsupportedAlgorithm(String),
    #[error("Random generation failed: {0}")]
    RandomError(String),
}

/// Key pair for signing
pub struct KeyPair {
    signing_key: SigningKey,
    algorithm: SignatureAlgorithm,
    key_id: Option<String>,
}

impl KeyPair {
    /// Generate a new Ed25519 key pair
    pub fn generate_ed25519() -> Result<Self, CryptoError> {
        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        Ok(Self {
            signing_key,
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: None,
        })
    }

    /// Generate with a specific key ID
    pub fn generate_ed25519_with_id(key_id: impl Into<String>) -> Result<Self, CryptoError> {
        let mut keypair = Self::generate_ed25519()?;
        keypair.key_id = Some(key_id.into());
        Ok(keypair)
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    /// Get the verifying key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> Signature {
        let sig = self.signing_key.sign(data);
        Signature {
            algorithm: self.algorithm,
            public_key: self.public_key_bytes(),
            signature: sig.to_bytes().to_vec(),
            signed_at: Timestamp::now(),
            key_id: self.key_id.clone(),
        }
    }
}

/// Verify a signature
pub fn verify_signature(signature: &Signature, data: &[u8]) -> Result<bool, CryptoError> {
    match signature.algorithm {
        SignatureAlgorithm::Ed25519 => {
            let public_key_bytes: [u8; 32] = signature
                .public_key
                .as_slice()
                .try_into()
                .map_err(|_| CryptoError::InvalidKeyLength)?;

            let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
                .map_err(|_| CryptoError::InvalidKeyLength)?;

            let sig_bytes: [u8; 64] = signature
                .signature
                .as_slice()
                .try_into()
                .map_err(|_| CryptoError::InvalidKeyLength)?;

            let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

            Ok(verifying_key.verify_strict(data, &sig).is_ok())
        }
        _ => Err(CryptoError::UnsupportedAlgorithm(format!(
            "{:?}",
            signature.algorithm
        ))),
    }
}

/// Compute hash of data
pub fn hash(algorithm: HashAlgorithm, data: &[u8]) -> Hash {
    let value = match algorithm {
        HashAlgorithm::Sha3_256 => {
            let mut hasher = Sha3_256::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Sha3_512 => {
            let mut hasher = Sha3_512::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        HashAlgorithm::Blake3 => {
            // Note: Would need blake3 crate for real implementation
            // For now, fall back to SHA3-256
            let mut hasher = Sha3_256::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
    };

    Hash { algorithm, value }
}

/// Compute SHA3-256 hash (convenience function)
pub fn sha3_256(data: &[u8]) -> Hash {
    hash(HashAlgorithm::Sha3_256, data)
}

/// Generate random bytes
pub fn random_bytes(len: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_sign_verify() {
        let keypair = KeyPair::generate_ed25519().unwrap();
        let data = b"Hello, CHINJU!";

        let signature = keypair.sign(data);
        assert!(verify_signature(&signature, data).unwrap());

        // Tampered data should fail
        let tampered = b"Hello, HACKED!";
        assert!(!verify_signature(&signature, tampered).unwrap());
    }

    #[test]
    fn test_hash() {
        let data = b"test data";
        let h = sha3_256(data);
        assert_eq!(h.algorithm, HashAlgorithm::Sha3_256);
        assert_eq!(h.value.len(), 32);

        // Same data should produce same hash
        let h2 = sha3_256(data);
        assert_eq!(h.value, h2.value);

        // Different data should produce different hash
        let h3 = sha3_256(b"different data");
        assert_ne!(h.value, h3.value);
    }
}
