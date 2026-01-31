//! Enclave communication protocol
//!
//! This module defines the same protocol as chinju-core/hardware/nitro/protocol.rs
//! to ensure compatibility between parent and Enclave.

use serde::{Deserialize, Serialize};

/// Request from parent to Enclave
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnclaveRequest {
    /// Get attestation document
    GetAttestation {
        challenge: Vec<u8>,
        user_data: Option<Vec<u8>>,
    },
    /// Seal data
    Seal { data: Vec<u8> },
    /// Unseal data
    Unseal { sealed_data: Vec<u8> },
    /// Sign data
    Sign { key_id: String, data: Vec<u8> },
    /// Generate key pair
    GenerateKeyPair { algorithm: String, label: String },
    /// Get public key
    GetPublicKey { key_id: String },
    /// Delete key
    DeleteKey { key_id: String },
    /// Health check
    HealthCheck,
    /// Heartbeat
    Heartbeat,
    /// Get status
    GetStatus,
}

/// Response from Enclave to parent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnclaveResponse {
    Attestation { document: Vec<u8> },
    Sealed { sealed_data: Vec<u8> },
    Unsealed { data: Vec<u8> },
    Signature { signature: Vec<u8>, public_key: Vec<u8> },
    KeyGenerated { key_id: String, public_key: Vec<u8> },
    PublicKey { key_id: String, public_key: Vec<u8> },
    KeyDeleted { key_id: String },
    Health { healthy: bool, version: String, uptime_seconds: u64 },
    HeartbeatAck { timestamp: u64 },
    Status { version: String, key_count: usize, memory_used: u64, uptime_seconds: u64 },
    Error { code: String, message: String },
}

impl EnclaveResponse {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}
