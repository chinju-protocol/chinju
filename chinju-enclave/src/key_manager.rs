//! Enclave Key Manager
//!
//! Manages cryptographic keys inside the Enclave.
//! Keys are stored in memory and destroyed when the Enclave terminates.

use crate::EnclaveError;
use ed25519_dalek::{Signature, Signer, SigningKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// Key entry in the key store
#[allow(dead_code)]
struct KeyEntry {
    /// Key algorithm
    algorithm: String,
    /// Human-readable label
    label: String,
    /// Signing key (Ed25519)
    signing_key: SigningKey,
    /// Creation timestamp
    created_at: u64,
}

/// Key manager for Enclave
pub struct KeyManager {
    /// Key store (key_id -> KeyEntry)
    keys: Arc<RwLock<HashMap<String, KeyEntry>>>,
    /// Sealing key (derived from NSM or generated)
    sealing_key: [u8; 32],
    /// Start time
    start_time: std::time::Instant,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new() -> Self {
        // Generate a sealing key (in real implementation, would derive from NSM)
        let mut sealing_key = [0u8; 32];
        rand::RngCore::fill_bytes(&mut OsRng, &mut sealing_key);

        info!("KeyManager initialized");

        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            sealing_key,
            start_time: std::time::Instant::now(),
        }
    }

    /// Generate a new key pair
    pub fn generate_key_pair(
        &self,
        algorithm: &str,
        label: &str,
    ) -> Result<(String, Vec<u8>), EnclaveError> {
        if algorithm != "Ed25519" {
            return Err(EnclaveError::KeyManagerError(format!(
                "Unsupported algorithm: {}. Only Ed25519 is supported.",
                algorithm
            )));
        }

        // Generate Ed25519 key pair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key = verifying_key.to_bytes().to_vec();

        // Generate key ID from public key hash
        let mut hasher = Sha256::new();
        hasher.update(&public_key);
        let hash = hasher.finalize();
        let key_id = format!("key-{}", hex::encode(&hash[..8]));

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| EnclaveError::KeyManagerError(e.to_string()))?
            .as_secs();

        let entry = KeyEntry {
            algorithm: algorithm.to_string(),
            label: label.to_string(),
            signing_key,
            created_at: now,
        };

        // Store key
        let mut keys = self
            .keys
            .write()
            .map_err(|e| EnclaveError::KeyManagerError(e.to_string()))?;
        keys.insert(key_id.clone(), entry);

        info!("Generated key: {} ({})", key_id, label);

        Ok((key_id, public_key))
    }

    /// Sign data with a key
    pub fn sign(&self, key_id: &str, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), EnclaveError> {
        let keys = self
            .keys
            .read()
            .map_err(|e| EnclaveError::KeyManagerError(e.to_string()))?;

        let entry = keys
            .get(key_id)
            .ok_or_else(|| EnclaveError::KeyManagerError(format!("Key not found: {}", key_id)))?;

        let signature: Signature = entry.signing_key.sign(data);
        let public_key = entry.signing_key.verifying_key().to_bytes().to_vec();

        debug!("Signed {} bytes with key {}", data.len(), key_id);

        Ok((signature.to_bytes().to_vec(), public_key))
    }

    /// Get public key for a key ID
    pub fn get_public_key(&self, key_id: &str) -> Result<Vec<u8>, EnclaveError> {
        let keys = self
            .keys
            .read()
            .map_err(|e| EnclaveError::KeyManagerError(e.to_string()))?;

        let entry = keys
            .get(key_id)
            .ok_or_else(|| EnclaveError::KeyManagerError(format!("Key not found: {}", key_id)))?;

        Ok(entry.signing_key.verifying_key().to_bytes().to_vec())
    }

    /// Delete a key
    pub fn delete_key(&self, key_id: &str) -> Result<(), EnclaveError> {
        let mut keys = self
            .keys
            .write()
            .map_err(|e| EnclaveError::KeyManagerError(e.to_string()))?;

        if keys.remove(key_id).is_some() {
            info!("Deleted key: {}", key_id);
            Ok(())
        } else {
            Err(EnclaveError::KeyManagerError(format!(
                "Key not found: {}",
                key_id
            )))
        }
    }

    /// Seal data (encrypt with sealing key)
    pub fn seal(&self, data: &[u8]) -> Result<Vec<u8>, EnclaveError> {
        // Simple XOR-based sealing (for demo; real implementation would use AES-GCM)
        // In production, this should use KMS with attestation-based access control
        let mut sealed = Vec::with_capacity(data.len() + 16);

        // Add a random nonce
        let mut nonce = [0u8; 16];
        rand::RngCore::fill_bytes(&mut OsRng, &mut nonce);
        sealed.extend_from_slice(&nonce);

        // Derive key from sealing key and nonce
        let mut hasher = Sha256::new();
        hasher.update(self.sealing_key);
        hasher.update(nonce);
        let derived_key = hasher.finalize();

        // XOR encrypt (simplified; use AES-GCM in production)
        for (i, byte) in data.iter().enumerate() {
            sealed.push(byte ^ derived_key[i % 32]);
        }

        debug!("Sealed {} bytes -> {} bytes", data.len(), sealed.len());

        Ok(sealed)
    }

    /// Unseal data (decrypt with sealing key)
    pub fn unseal(&self, sealed_data: &[u8]) -> Result<Vec<u8>, EnclaveError> {
        if sealed_data.len() < 16 {
            return Err(EnclaveError::KeyManagerError(
                "Sealed data too short".to_string(),
            ));
        }

        // Extract nonce
        let nonce = &sealed_data[..16];
        let encrypted = &sealed_data[16..];

        // Derive key from sealing key and nonce
        let mut hasher = Sha256::new();
        hasher.update(self.sealing_key);
        hasher.update(nonce);
        let derived_key = hasher.finalize();

        // XOR decrypt
        let mut data = Vec::with_capacity(encrypted.len());
        for (i, byte) in encrypted.iter().enumerate() {
            data.push(byte ^ derived_key[i % 32]);
        }

        debug!(
            "Unsealed {} bytes -> {} bytes",
            sealed_data.len(),
            data.len()
        );

        Ok(data)
    }

    /// Get key count
    pub fn key_count(&self) -> usize {
        self.keys.read().map(|k| k.len()).unwrap_or(0)
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for KeyManager {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            sealing_key: self.sealing_key,
            start_time: self.start_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::VerifyingKey;

    #[test]
    fn test_key_generation() {
        let km = KeyManager::new();
        let (key_id, public_key) = km.generate_key_pair("Ed25519", "test-key").unwrap();

        assert!(key_id.starts_with("key-"));
        assert_eq!(public_key.len(), 32); // Ed25519 public key is 32 bytes
    }

    #[test]
    fn test_sign_and_verify() {
        let km = KeyManager::new();
        let (key_id, _) = km.generate_key_pair("Ed25519", "test-key").unwrap();

        let data = b"Hello, World!";
        let (signature, public_key) = km.sign(&key_id, data).unwrap();

        assert_eq!(signature.len(), 64); // Ed25519 signature is 64 bytes

        // Verify signature
        let verifying_key = VerifyingKey::from_bytes(&public_key.try_into().unwrap()).unwrap();
        let sig = Signature::from_bytes(&signature.try_into().unwrap());
        assert!(verifying_key.verify_strict(data, &sig).is_ok());
    }

    #[test]
    fn test_seal_unseal() {
        let km = KeyManager::new();

        let plaintext = b"Secret data";
        let sealed = km.seal(plaintext).unwrap();
        let unsealed = km.unseal(&sealed).unwrap();

        assert_eq!(unsealed, plaintext);
    }

    #[test]
    fn test_delete_key() {
        let km = KeyManager::new();
        let (key_id, _) = km.generate_key_pair("Ed25519", "test-key").unwrap();

        assert_eq!(km.key_count(), 1);
        km.delete_key(&key_id).unwrap();
        assert_eq!(km.key_count(), 0);
    }
}
