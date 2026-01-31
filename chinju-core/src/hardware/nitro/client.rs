//! Nitro Enclave client
//!
//! High-level client for communicating with a Nitro Enclave.
//! Provides typed methods for common operations.

use super::attestation::{AttestationDocument, AttestationVerificationConfig, AttestationVerifier};
use super::error::NitroError;
use super::protocol::{EnclaveRequest, EnclaveResponse};
use super::vsock::{VsockClient, VsockConfig};
use tracing::{debug, info};

/// High-level client for Nitro Enclave operations
pub struct NitroEnclaveClient {
    vsock: VsockClient,
    verifier: AttestationVerifier,
    debug_mode: bool,
}

impl NitroEnclaveClient {
    /// Create a new client with the given configuration
    pub fn new(vsock_config: VsockConfig) -> Result<Self, NitroError> {
        let debug_mode = std::env::var("CHINJU_NITRO_DEBUG")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let verifier = if debug_mode {
            AttestationVerifier::debug_mode()
        } else {
            AttestationVerifier::from_env()?
        };

        info!(
            "NitroEnclaveClient initialized (debug_mode={}, cid={}, port={})",
            debug_mode, vsock_config.cid, vsock_config.port
        );

        Ok(Self {
            vsock: VsockClient::new(vsock_config),
            verifier,
            debug_mode,
        })
    }

    /// Create a client from environment variables
    pub fn from_env() -> Result<Self, NitroError> {
        let vsock_config = VsockConfig::from_env()?;
        Self::new(vsock_config)
    }

    /// Create a client with custom attestation verification config
    pub fn with_verification_config(
        vsock_config: VsockConfig,
        attestation_config: AttestationVerificationConfig,
    ) -> Result<Self, NitroError> {
        let debug_mode = attestation_config.allow_debug;
        let verifier = AttestationVerifier::new(attestation_config);

        Ok(Self {
            vsock: VsockClient::new(vsock_config),
            verifier,
            debug_mode,
        })
    }

    /// Check if running in debug mode
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode
    }

    /// Get attestation document from the Enclave
    ///
    /// # Arguments
    /// * `challenge` - Random bytes for freshness (nonce)
    /// * `user_data` - Optional user data to include in attestation
    ///
    /// # Returns
    /// Raw CBOR-encoded attestation document
    pub fn get_attestation(
        &self,
        challenge: &[u8],
        user_data: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, NitroError> {
        debug!("Requesting attestation with {} byte challenge", challenge.len());

        let request = EnclaveRequest::GetAttestation {
            challenge: challenge.to_vec(),
            user_data,
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::Attestation { document } => Ok(document),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for attestation".to_string(),
            )),
        }
    }

    /// Get and verify attestation document
    ///
    /// # Arguments
    /// * `challenge` - Random bytes for freshness (nonce)
    /// * `user_data` - Optional user data to include in attestation
    ///
    /// # Returns
    /// Verified attestation document
    pub fn get_verified_attestation(
        &self,
        challenge: &[u8],
        user_data: Option<Vec<u8>>,
    ) -> Result<AttestationDocument, NitroError> {
        let raw_document = self.get_attestation(challenge, user_data)?;
        self.verifier.verify(&raw_document, challenge)
    }

    /// Seal data using Enclave-bound encryption key
    ///
    /// Sealed data can only be unsealed by the same Enclave (or an Enclave
    /// with matching PCR values).
    pub fn seal(&self, data: &[u8]) -> Result<Vec<u8>, NitroError> {
        debug!("Sealing {} bytes", data.len());

        let request = EnclaveRequest::Seal {
            data: data.to_vec(),
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::Sealed { sealed_data } => Ok(sealed_data),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for seal".to_string(),
            )),
        }
    }

    /// Unseal previously sealed data
    pub fn unseal(&self, sealed_data: &[u8]) -> Result<Vec<u8>, NitroError> {
        debug!("Unsealing {} bytes", sealed_data.len());

        let request = EnclaveRequest::Unseal {
            sealed_data: sealed_data.to_vec(),
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::Unsealed { data } => Ok(data),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for unseal".to_string(),
            )),
        }
    }

    /// Sign data with an Enclave key
    pub fn sign(&self, key_id: &str, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), NitroError> {
        debug!("Signing {} bytes with key '{}'", data.len(), key_id);

        let request = EnclaveRequest::Sign {
            key_id: key_id.to_string(),
            data: data.to_vec(),
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::Signature {
                signature,
                public_key,
            } => Ok((signature, public_key)),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for sign".to_string(),
            )),
        }
    }

    /// Generate a new key pair in the Enclave
    pub fn generate_key_pair(
        &self,
        algorithm: &str,
        label: &str,
    ) -> Result<(String, Vec<u8>), NitroError> {
        debug!("Generating {} key pair with label '{}'", algorithm, label);

        let request = EnclaveRequest::GenerateKeyPair {
            algorithm: algorithm.to_string(),
            label: label.to_string(),
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::KeyGenerated { key_id, public_key } => Ok((key_id, public_key)),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for generate_key_pair".to_string(),
            )),
        }
    }

    /// Get public key for an existing key
    pub fn get_public_key(&self, key_id: &str) -> Result<Vec<u8>, NitroError> {
        debug!("Getting public key for '{}'", key_id);

        let request = EnclaveRequest::GetPublicKey {
            key_id: key_id.to_string(),
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::PublicKey { public_key, .. } => Ok(public_key),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for get_public_key".to_string(),
            )),
        }
    }

    /// Delete a key from the Enclave
    pub fn delete_key(&self, key_id: &str) -> Result<(), NitroError> {
        debug!("Deleting key '{}'", key_id);

        let request = EnclaveRequest::DeleteKey {
            key_id: key_id.to_string(),
        };

        let response = self.vsock.send(request)?;

        match response {
            EnclaveResponse::KeyDeleted { .. } => Ok(()),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for delete_key".to_string(),
            )),
        }
    }

    /// Check Enclave health
    pub fn health_check(&self) -> Result<(bool, String, u64), NitroError> {
        debug!("Performing health check");

        let response = self.vsock.send(EnclaveRequest::HealthCheck)?;

        match response {
            EnclaveResponse::Health {
                healthy,
                version,
                uptime_seconds,
            } => Ok((healthy, version, uptime_seconds)),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for health_check".to_string(),
            )),
        }
    }

    /// Send heartbeat to Enclave (for Dead Man's Switch)
    pub fn heartbeat(&self) -> Result<u64, NitroError> {
        self.vsock.heartbeat()
    }

    /// Get Enclave status
    pub fn get_status(&self) -> Result<EnclaveStatus, NitroError> {
        debug!("Getting Enclave status");

        let response = self.vsock.send(EnclaveRequest::GetStatus)?;

        match response {
            EnclaveResponse::Status {
                version,
                key_count,
                memory_used,
                uptime_seconds,
            } => Ok(EnclaveStatus {
                version,
                key_count,
                memory_used,
                uptime_seconds,
            }),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type for get_status".to_string(),
            )),
        }
    }
}

/// Enclave status information
#[derive(Debug, Clone)]
pub struct EnclaveStatus {
    /// Enclave application version
    pub version: String,
    /// Number of keys stored in Enclave
    pub key_count: usize,
    /// Memory usage in bytes
    pub memory_used: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enclave_status() {
        let status = EnclaveStatus {
            version: "1.0.0".to_string(),
            key_count: 5,
            memory_used: 1024 * 1024,
            uptime_seconds: 3600,
        };

        assert_eq!(status.version, "1.0.0");
        assert_eq!(status.key_count, 5);
    }
}
