//! Mock HSM implementation
//!
//! Software-based HSM for development and testing.
//! WARNING: NOT suitable for production use.

use crate::crypto::{verify_signature, KeyPair};
use crate::hardware::traits::*;
use crate::types::{HardwareAttestation, Signature, Timestamp, TrustLevel};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::warn;

/// Mock HSM implementation
///
/// Stores keys in memory. All keys are lost when the process exits.
/// This is intentional for testing - production should use real HSM.
pub struct MockHsm {
    keys: Arc<RwLock<HashMap<String, KeyPair>>>,
    next_id: Arc<RwLock<u64>>,
}

impl MockHsm {
    /// Create a new mock HSM
    pub fn new() -> Self {
        warn!("{}", super::MOCK_WARNING);
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    fn generate_key_id(&self) -> String {
        let mut id = self.next_id.write().unwrap();
        let key_id = format!("mock-key-{:08x}", *id);
        *id += 1;
        key_id
    }
}

impl Default for MockHsm {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustRoot for MockHsm {
    fn is_hardware_backed(&self) -> bool {
        false // This is a mock
    }

    fn security_level(&self) -> TrustLevel {
        TrustLevel::Mock
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        Ok(HardwareAttestation {
            trust_level: TrustLevel::Mock,
            hardware_type: "MockHSM".to_string(),
            attestation_data: b"MOCK_ATTESTATION_NOT_FOR_PRODUCTION".to_vec(),
            manufacturer_signature: None,
            attested_at: Timestamp::now(),
        })
    }
}

impl HardwareSecurityModule for MockHsm {
    fn generate_key_pair(
        &self,
        algorithm: KeyAlgorithm,
        label: &str,
    ) -> Result<KeyHandle, HardwareError> {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                let key_id = self.generate_key_id();
                let keypair = KeyPair::generate_ed25519_with_id(&key_id)
                    .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

                let mut keys = self.keys.write().unwrap();
                keys.insert(key_id.clone(), keypair);

                tracing::info!(key_id = %key_id, label = %label, "Generated mock key pair");
                Ok(KeyHandle::new(key_id))
            }
            _ => Err(HardwareError::NotSupported),
        }
    }

    fn sign(&self, key_handle: &KeyHandle, data: &[u8]) -> Result<Signature, HardwareError> {
        let keys = self.keys.read().unwrap();
        let keypair = keys
            .get(&key_handle.0)
            .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?;

        Ok(keypair.sign(data))
    }

    fn verify(
        &self,
        key_handle: &KeyHandle,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, HardwareError> {
        // First check if we have this key
        if !self.key_exists(key_handle) {
            return Err(HardwareError::KeyNotFound(key_handle.0.clone()));
        }

        // Verify the signature
        verify_signature(signature, data).map_err(|e| HardwareError::InvalidData(e.to_string()))
    }

    fn get_public_key(&self, key_handle: &KeyHandle) -> Result<Vec<u8>, HardwareError> {
        let keys = self.keys.read().unwrap();
        let keypair = keys
            .get(&key_handle.0)
            .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?;

        Ok(keypair.public_key_bytes())
    }

    fn secure_erase(&self, key_handle: &KeyHandle) -> Result<(), HardwareError> {
        let mut keys = self.keys.write().unwrap();
        if keys.remove(&key_handle.0).is_some() {
            tracing::info!(key_id = %key_handle.0, "Securely erased mock key");
            Ok(())
        } else {
            Err(HardwareError::KeyNotFound(key_handle.0.clone()))
        }
    }

    fn key_exists(&self, key_handle: &KeyHandle) -> bool {
        let keys = self.keys.read().unwrap();
        keys.contains_key(&key_handle.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_hsm_basic_operations() {
        let hsm = MockHsm::new();

        // Generate key
        let key_handle = hsm
            .generate_key_pair(KeyAlgorithm::Ed25519, "test-key")
            .unwrap();

        assert!(hsm.key_exists(&key_handle));

        // Sign and verify
        let data = b"Hello, CHINJU!";
        let signature = hsm.sign(&key_handle, data).unwrap();
        assert!(hsm.verify(&key_handle, data, &signature).unwrap());

        // Tampered data should fail
        let tampered = b"Hello, HACKED!";
        assert!(!hsm.verify(&key_handle, tampered, &signature).unwrap());

        // Erase key
        hsm.secure_erase(&key_handle).unwrap();
        assert!(!hsm.key_exists(&key_handle));
    }

    #[test]
    fn test_mock_hsm_attestation() {
        let hsm = MockHsm::new();
        let attestation = hsm.get_attestation().unwrap();

        assert_eq!(attestation.trust_level, TrustLevel::Mock);
        assert_eq!(attestation.hardware_type, "MockHSM");
        assert!(!hsm.is_hardware_backed());
    }
}
