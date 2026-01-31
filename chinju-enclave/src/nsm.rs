//! Nitro Secure Module (NSM) API wrapper
//!
//! Provides access to the NSM device (/dev/nsm) for:
//! - Attestation document generation
//! - Random number generation
//!
//! In mock mode, simulates these operations for local development.

use crate::EnclaveError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Mock attestation document (for development)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationDocument {
    pub module_id: String,
    pub timestamp: u64,
    pub digest: String,
    pub pcrs: HashMap<usize, Vec<u8>>,
    pub certificate: Vec<u8>,
    pub cabundle: Vec<Vec<u8>>,
    pub user_data: Option<Vec<u8>>,
    pub nonce: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

/// NSM client
pub struct NsmClient {
    #[cfg(feature = "mock")]
    mock_mode: bool,
}

impl NsmClient {
    /// Create a new NSM client
    pub fn new() -> Result<Self, EnclaveError> {
        #[cfg(feature = "mock")]
        {
            debug!("NSM client running in mock mode");
            Ok(Self { mock_mode: true })
        }

        #[cfg(not(feature = "mock"))]
        {
            // In real mode, would open /dev/nsm
            // let fd = aws_nitro_enclaves_nsm_api::driver::nsm_init();
            // ...
            Err(EnclaveError::NsmError(
                "Real NSM mode not yet implemented".to_string(),
            ))
        }
    }

    /// Get attestation document
    pub fn get_attestation_document(
        &self,
        user_data: Option<&[u8]>,
        nonce: Option<&[u8]>,
        public_key: Option<&[u8]>,
    ) -> Result<Vec<u8>, EnclaveError> {
        #[cfg(feature = "mock")]
        {
            debug!("Generating mock attestation document");

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| EnclaveError::NsmError(e.to_string()))?
                .as_millis() as u64;

            // Create mock PCR values (all zeros in debug mode)
            let mut pcrs = HashMap::new();
            for i in 0..16 {
                pcrs.insert(i, vec![0u8; 48]);
            }

            let doc = AttestationDocument {
                module_id: "mock-enclave-001".to_string(),
                timestamp: now,
                digest: "SHA384".to_string(),
                pcrs,
                certificate: vec![0u8; 32], // Mock certificate
                cabundle: vec![vec![0u8; 32]], // Mock CA bundle
                user_data: user_data.map(|d| d.to_vec()),
                nonce: nonce.map(|d| d.to_vec()),
                public_key: public_key.map(|d| d.to_vec()),
            };

            serde_cbor::to_vec(&doc)
                .map_err(|e| EnclaveError::SerializationError(e.to_string()))
        }

        #[cfg(not(feature = "mock"))]
        {
            // Real NSM implementation would go here
            Err(EnclaveError::NsmError(
                "Real NSM mode not yet implemented".to_string(),
            ))
        }
    }

    /// Get random bytes from NSM
    pub fn get_random(&self, length: usize) -> Result<Vec<u8>, EnclaveError> {
        #[cfg(feature = "mock")]
        {
            use rand::RngCore;
            let mut bytes = vec![0u8; length];
            rand::thread_rng().fill_bytes(&mut bytes);
            Ok(bytes)
        }

        #[cfg(not(feature = "mock"))]
        {
            Err(EnclaveError::NsmError(
                "Real NSM mode not yet implemented".to_string(),
            ))
        }
    }
}

impl Default for NsmClient {
    fn default() -> Self {
        Self::new().expect("Failed to create NSM client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_attestation() {
        let client = NsmClient::new().unwrap();
        let nonce = vec![1, 2, 3, 4];
        let doc = client
            .get_attestation_document(None, Some(&nonce), None)
            .unwrap();
        assert!(!doc.is_empty());
    }

    #[test]
    fn test_mock_random() {
        let client = NsmClient::new().unwrap();
        let random = client.get_random(32).unwrap();
        assert_eq!(random.len(), 32);
    }
}
