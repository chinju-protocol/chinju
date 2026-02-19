//! Nitro Secure Module (NSM) API wrapper
//!
//! Provides access to the NSM device (/dev/nsm) for:
//! - Attestation document generation
//! - Random number generation
//!
//! In mock mode, simulates these operations for local development.

use crate::EnclaveError;
use tracing::debug;

#[cfg(all(feature = "mock", not(feature = "nsm")))]
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "mock", not(feature = "nsm")))]
use std::collections::HashMap;
#[cfg(all(feature = "mock", not(feature = "nsm")))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "nsm")]
use aws_nitro_enclaves_nsm_api::api::{ErrorCode, Request, Response};
#[cfg(feature = "nsm")]
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
#[cfg(feature = "nsm")]
use serde_bytes::ByteBuf;

/// Mock attestation document (for development)
#[cfg(all(feature = "mock", not(feature = "nsm")))]
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
    #[cfg(feature = "nsm")]
    fd: Option<i32>,
    #[cfg(all(feature = "mock", not(feature = "nsm")))]
    #[allow(dead_code)]
    mock_mode: bool,
}

impl NsmClient {
    /// Create a new NSM client
    pub fn new() -> Result<Self, EnclaveError> {
        #[cfg(feature = "nsm")]
        {
            let fd = nsm_init();
            if fd < 0 {
                return Err(EnclaveError::NsmError(
                    "Failed to initialize NSM device (/dev/nsm)".to_string(),
                ));
            }
            debug!(fd = fd, "NSM client initialized");
            return Ok(Self { fd: Some(fd) });
        }

        #[cfg(all(feature = "mock", not(feature = "nsm")))]
        {
            debug!("NSM client running in mock mode");
            Ok(Self { mock_mode: true })
        }

        #[cfg(not(any(feature = "nsm", feature = "mock")))]
        {
            Err(EnclaveError::NsmError(
                "No NSM backend enabled; enable `mock` or `nsm` feature".to_string(),
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
        #[cfg(feature = "nsm")]
        {
            let fd = self.fd.ok_or_else(|| {
                EnclaveError::NsmError("NSM client is not initialized".to_string())
            })?;

            let request = Request::Attestation {
                user_data: user_data.map(|v| ByteBuf::from(v.to_vec())),
                nonce: nonce.map(|v| ByteBuf::from(v.to_vec())),
                public_key: public_key.map(|v| ByteBuf::from(v.to_vec())),
            };

            return match nsm_process_request(fd, request) {
                Response::Attestation { document } => Ok(document),
                Response::Error(code) => Err(map_error_code(code)),
                other => Err(EnclaveError::NsmError(format!(
                    "Unexpected NSM response for attestation: {:?}",
                    other
                ))),
            };
        }

        #[cfg(all(feature = "mock", not(feature = "nsm")))]
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
                certificate: vec![0u8; 32],    // Mock certificate
                cabundle: vec![vec![0u8; 32]], // Mock CA bundle
                user_data: user_data.map(|d| d.to_vec()),
                nonce: nonce.map(|d| d.to_vec()),
                public_key: public_key.map(|d| d.to_vec()),
            };

            serde_cbor::to_vec(&doc).map_err(|e| EnclaveError::SerializationError(e.to_string()))
        }

        #[cfg(not(any(feature = "nsm", feature = "mock")))]
        {
            Err(EnclaveError::NsmError(
                "No NSM backend enabled; enable `mock` or `nsm` feature".to_string(),
            ))
        }
    }

    /// Get random bytes from NSM
    pub fn get_random(&self, length: usize) -> Result<Vec<u8>, EnclaveError> {
        #[cfg(feature = "nsm")]
        {
            let fd = self.fd.ok_or_else(|| {
                EnclaveError::NsmError("NSM client is not initialized".to_string())
            })?;
            let mut out = Vec::with_capacity(length);

            while out.len() < length {
                match nsm_process_request(fd, Request::GetRandom) {
                    Response::GetRandom { random } => out.extend_from_slice(&random),
                    Response::Error(code) => return Err(map_error_code(code)),
                    other => {
                        return Err(EnclaveError::NsmError(format!(
                            "Unexpected NSM response for GetRandom: {:?}",
                            other
                        )));
                    }
                }
            }
            out.truncate(length);
            return Ok(out);
        }

        #[cfg(all(feature = "mock", not(feature = "nsm")))]
        {
            use rand::RngCore;
            let mut bytes = vec![0u8; length];
            rand::thread_rng().fill_bytes(&mut bytes);
            Ok(bytes)
        }

        #[cfg(not(any(feature = "nsm", feature = "mock")))]
        {
            Err(EnclaveError::NsmError(
                "No NSM backend enabled; enable `mock` or `nsm` feature".to_string(),
            ))
        }
    }
}

impl Default for NsmClient {
    fn default() -> Self {
        match Self::new() {
            Ok(client) => client,
            Err(e) => {
                tracing::error!("Failed to create NSM client: {}", e);
                #[cfg(feature = "nsm")]
                {
                    Self { fd: None }
                }
                #[cfg(all(feature = "mock", not(feature = "nsm")))]
                {
                    Self { mock_mode: true }
                }
                #[cfg(not(any(feature = "nsm", feature = "mock")))]
                {
                    panic!("No NSM backend is available");
                }
            }
        }
    }
}

#[cfg(feature = "nsm")]
impl Drop for NsmClient {
    fn drop(&mut self) {
        if let Some(fd) = self.fd.take() {
            nsm_exit(fd);
        }
    }
}

#[cfg(feature = "nsm")]
fn map_error_code(code: ErrorCode) -> EnclaveError {
    EnclaveError::NsmError(format!("NSM error: {:?}", code))
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
