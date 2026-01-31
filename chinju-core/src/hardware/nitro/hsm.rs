//! NitroHsm - SecureExecution implementation using AWS Nitro Enclaves
//!
//! Provides L3 (Enterprise) level secure execution for CHINJU Protocol.

use super::attestation::AttestationVerificationConfig;
use super::client::NitroEnclaveClient;
use super::error::NitroError;
use super::vsock::VsockConfig;
use crate::hardware::traits::{HardwareError, SecureExecution, TrustRoot};
use crate::types::{HardwareAttestation, Timestamp, TrustLevel};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Nitro Enclave HSM configuration
#[derive(Debug, Clone)]
pub struct NitroConfig {
    /// vsock communication settings
    pub vsock: VsockConfig,
    /// Attestation verification settings
    pub attestation: AttestationVerificationConfig,
    /// Enable debug mode (disables PCR verification)
    pub debug_mode: bool,
}

impl NitroConfig {
    /// Create configuration from environment variables
    ///
    /// # Environment Variables
    ///
    /// Required:
    /// - `CHINJU_NITRO_ENCLAVE_CID`: Enclave CID
    ///
    /// Optional:
    /// - `CHINJU_NITRO_PORT`: vsock port (default: 5000)
    /// - `CHINJU_NITRO_DEBUG`: Enable debug mode (default: false)
    /// - `CHINJU_NITRO_PCR0`: Expected PCR0 value (hex)
    /// - `CHINJU_NITRO_PCR1`: Expected PCR1 value (hex)
    /// - `CHINJU_NITRO_PCR2`: Expected PCR2 value (hex)
    pub fn from_env() -> Result<Self, NitroError> {
        let vsock = VsockConfig::from_env()?;

        let debug_mode = std::env::var("CHINJU_NITRO_DEBUG")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if debug_mode {
            warn!("NitroHsm running in DEBUG mode - PCR verification disabled");
        }

        let attestation = if debug_mode {
            AttestationVerificationConfig::debug()
        } else {
            AttestationVerificationConfig::from_env()?
        };

        Ok(Self {
            vsock,
            attestation,
            debug_mode,
        })
    }

    /// Create a debug configuration for testing
    pub fn debug(cid: u32, port: u32) -> Self {
        Self {
            vsock: VsockConfig::new(cid, port),
            attestation: AttestationVerificationConfig::debug(),
            debug_mode: true,
        }
    }
}

/// Nitro Enclave HSM implementation
///
/// Provides hardware-backed secure execution using AWS Nitro Enclaves.
/// Implements `TrustRoot` and `SecureExecution` traits.
pub struct NitroHsm {
    client: Arc<NitroEnclaveClient>,
    config: NitroConfig,
    /// Cached attestation (valid for short period)
    cached_attestation: RwLock<Option<CachedAttestation>>,
}

struct CachedAttestation {
    attestation: HardwareAttestation,
    timestamp: Timestamp,
}

impl NitroHsm {
    /// Create a new NitroHsm instance
    pub fn new(config: NitroConfig) -> Result<Self, HardwareError> {
        let client = NitroEnclaveClient::with_verification_config(
            config.vsock.clone(),
            config.attestation.clone(),
        )
        .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;

        info!(
            "NitroHsm initialized (debug_mode={}, cid={}, port={})",
            config.debug_mode, config.vsock.cid, config.vsock.port
        );

        Ok(Self {
            client: Arc::new(client),
            config,
            cached_attestation: RwLock::new(None),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, HardwareError> {
        let config = NitroConfig::from_env()
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;
        Self::new(config)
    }

    /// Get the underlying client
    pub fn client(&self) -> &NitroEnclaveClient {
        &self.client
    }

    /// Check if Enclave is healthy
    pub fn health_check(&self) -> Result<bool, HardwareError> {
        self.client
            .health_check()
            .map(|(healthy, _, _)| healthy)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    /// Send heartbeat to Enclave
    pub fn heartbeat(&self) -> Result<u64, HardwareError> {
        self.client
            .heartbeat()
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    /// Sign data using Enclave key
    pub fn sign(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>, HardwareError> {
        let (signature, _) = self
            .client
            .sign(key_id, data)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;
        Ok(signature)
    }

    /// Generate a new key pair
    pub fn generate_key_pair(
        &self,
        algorithm: &str,
        label: &str,
    ) -> Result<(String, Vec<u8>), HardwareError> {
        self.client
            .generate_key_pair(algorithm, label)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    /// Get public key
    pub fn get_public_key(&self, key_id: &str) -> Result<Vec<u8>, HardwareError> {
        self.client
            .get_public_key(key_id)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    /// Delete a key
    pub fn delete_key(&self, key_id: &str) -> Result<(), HardwareError> {
        self.client
            .delete_key(key_id)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }
}

impl TrustRoot for NitroHsm {
    fn is_hardware_backed(&self) -> bool {
        // Nitro Enclaves are always hardware-backed
        true
    }

    fn security_level(&self) -> TrustLevel {
        if self.config.debug_mode {
            // Debug mode is L1 (Software)
            TrustLevel::Software
        } else {
            // Production Nitro is L3 (Enterprise)
            TrustLevel::HardwareEnterprise
        }
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        // Check cache (valid for 60 seconds)
        if let Ok(guard) = self.cached_attestation.try_read() {
            if let Some(cached) = guard.as_ref() {
                let now = Timestamp::now();
                if now.seconds - cached.timestamp.seconds < 60 {
                    debug!("Returning cached attestation");
                    return Ok(cached.attestation.clone());
                }
            }
        }

        // Generate fresh attestation
        let challenge = rand::random::<[u8; 32]>().to_vec();

        let raw_document = self
            .client
            .get_attestation(&challenge, None)
            .map_err(|e| HardwareError::AttestationFailed(e.to_string()))?;

        let attestation = HardwareAttestation {
            trust_level: self.security_level(),
            hardware_type: "AWS Nitro Enclave".to_string(),
            attestation_data: raw_document,
            manufacturer_signature: None, // Attestation document is self-signed
            attested_at: Timestamp::now(),
        };

        // Update cache
        if let Ok(mut guard) = self.cached_attestation.try_write() {
            *guard = Some(CachedAttestation {
                attestation: attestation.clone(),
                timestamp: Timestamp::now(),
            });
        }

        Ok(attestation)
    }
}

impl SecureExecution for NitroHsm {
    fn execute_secure<F, R>(&self, _f: F) -> Result<R, HardwareError>
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        // Direct closure execution is not supported in Nitro Enclaves
        // Use seal/unseal or sign operations instead
        Err(HardwareError::NotSupported)
    }

    fn seal_data(&self, data: &[u8]) -> Result<Vec<u8>, HardwareError> {
        self.client
            .seal(data)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    fn unseal_data(&self, sealed: &[u8]) -> Result<Vec<u8>, HardwareError> {
        self.client
            .unseal(sealed)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    fn remote_attestation(&self, challenge: &[u8]) -> Result<Vec<u8>, HardwareError> {
        self.client
            .get_attestation(challenge, None)
            .map_err(|e| HardwareError::AttestationFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nitro_config_debug() {
        let config = NitroConfig::debug(16, 5000);
        assert!(config.debug_mode);
        assert_eq!(config.vsock.cid, 16);
        assert_eq!(config.vsock.port, 5000);
    }
}
