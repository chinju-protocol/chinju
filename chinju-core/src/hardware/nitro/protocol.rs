//! Parent-Enclave communication protocol
//!
//! Defines the request/response messages exchanged between the
//! EC2 parent instance and the Nitro Enclave via vsock.

use serde::{Deserialize, Serialize};

/// Request from parent to Enclave
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnclaveRequest {
    /// Get attestation document
    GetAttestation {
        /// Challenge bytes (nonce) for freshness
        challenge: Vec<u8>,
        /// Optional user data to include in attestation
        user_data: Option<Vec<u8>>,
    },

    /// Seal data using Enclave key
    Seal {
        /// Plaintext data to seal
        data: Vec<u8>,
    },

    /// Unseal previously sealed data
    Unseal {
        /// Sealed data to unseal
        sealed_data: Vec<u8>,
    },

    /// Sign data with Enclave key
    Sign {
        /// Key identifier
        key_id: String,
        /// Data to sign
        data: Vec<u8>,
    },

    /// Generate a new key pair
    GenerateKeyPair {
        /// Key algorithm (e.g., "Ed25519", "EcdsaP256")
        algorithm: String,
        /// Human-readable label
        label: String,
    },

    /// Get public key for a key handle
    GetPublicKey {
        /// Key identifier
        key_id: String,
    },

    /// Delete a key
    DeleteKey {
        /// Key identifier
        key_id: String,
    },

    /// Health check
    HealthCheck,

    /// Heartbeat for Dead Man's Switch
    Heartbeat,

    /// Get Enclave status/info
    GetStatus,
}

/// Response from Enclave to parent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnclaveResponse {
    /// Attestation document
    Attestation {
        /// CBOR-encoded attestation document
        document: Vec<u8>,
    },

    /// Sealed data result
    Sealed {
        /// Sealed (encrypted) data
        sealed_data: Vec<u8>,
    },

    /// Unsealed data result
    Unsealed {
        /// Plaintext data
        data: Vec<u8>,
    },

    /// Signature result
    Signature {
        /// Signature bytes
        signature: Vec<u8>,
        /// Public key used for signing
        public_key: Vec<u8>,
    },

    /// Key generation result
    KeyGenerated {
        /// Generated key identifier
        key_id: String,
        /// Public key bytes
        public_key: Vec<u8>,
    },

    /// Public key result
    PublicKey {
        /// Key identifier
        key_id: String,
        /// Public key bytes
        public_key: Vec<u8>,
    },

    /// Key deletion result
    KeyDeleted {
        /// Deleted key identifier
        key_id: String,
    },

    /// Health check result
    Health {
        /// Whether Enclave is healthy
        healthy: bool,
        /// Enclave version
        version: String,
        /// Uptime in seconds
        uptime_seconds: u64,
    },

    /// Heartbeat acknowledgment
    HeartbeatAck {
        /// Server timestamp
        timestamp: u64,
    },

    /// Enclave status
    Status {
        /// Enclave version
        version: String,
        /// Number of active keys
        key_count: usize,
        /// Memory usage in bytes
        memory_used: u64,
        /// Uptime in seconds
        uptime_seconds: u64,
    },

    /// Error response
    Error {
        /// Error code
        code: String,
        /// Error message
        message: String,
    },
}

impl EnclaveResponse {
    /// Create an error response
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Extract error if this is an error response
    pub fn into_error(self) -> Option<(String, String)> {
        match self {
            Self::Error { code, message } => Some((code, message)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = EnclaveRequest::GetAttestation {
            challenge: vec![1, 2, 3, 4],
            user_data: Some(vec![5, 6, 7, 8]),
        };

        let bytes = serde_cbor::to_vec(&request).unwrap();
        let decoded: EnclaveRequest = serde_cbor::from_slice(&bytes).unwrap();

        match decoded {
            EnclaveRequest::GetAttestation {
                challenge,
                user_data,
            } => {
                assert_eq!(challenge, vec![1, 2, 3, 4]);
                assert_eq!(user_data, Some(vec![5, 6, 7, 8]));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_response_serialization() {
        let response = EnclaveResponse::Health {
            healthy: true,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
        };

        let bytes = serde_cbor::to_vec(&response).unwrap();
        let decoded: EnclaveResponse = serde_cbor::from_slice(&bytes).unwrap();

        match decoded {
            EnclaveResponse::Health {
                healthy,
                version,
                uptime_seconds,
            } => {
                assert!(healthy);
                assert_eq!(version, "1.0.0");
                assert_eq!(uptime_seconds, 3600);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_error_response() {
        let response = EnclaveResponse::error("INVALID_KEY", "Key not found");
        assert!(response.is_error());

        let (code, message) = response.into_error().unwrap();
        assert_eq!(code, "INVALID_KEY");
        assert_eq!(message, "Key not found");
    }
}
