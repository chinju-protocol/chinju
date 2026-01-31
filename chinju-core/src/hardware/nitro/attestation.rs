//! Nitro Attestation Document verification
//!
//! AWS Nitro Attestation Documents provide cryptographic proof
//! that code is running inside a genuine Nitro Enclave.
//!
//! # Document Structure
//!
//! The attestation document is a COSE Sign1 structure containing:
//! - PCR values (Platform Configuration Registers)
//! - Certificate chain (AWS -> Enclave)
//! - Timestamp
//! - Optional user data and nonce

use super::error::NitroError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// PCR (Platform Configuration Register) indices
pub mod pcr {
    /// PCR0: Enclave image file (EIF)
    pub const IMAGE: usize = 0;
    /// PCR1: Linux kernel and boot command
    pub const KERNEL: usize = 1;
    /// PCR2: Application
    pub const APPLICATION: usize = 2;
    /// PCR3: IAM role ARN (if used)
    pub const IAM_ROLE: usize = 3;
    /// PCR4: Instance ID
    pub const INSTANCE_ID: usize = 4;
    /// PCR8: Signing certificate (if used)
    pub const SIGNING_CERT: usize = 8;
}

/// Decoded Attestation Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationDocument {
    /// Module ID (Enclave identifier)
    pub module_id: String,
    /// Timestamp in milliseconds since UNIX epoch
    pub timestamp: u64,
    /// Digest algorithm (typically "SHA384")
    pub digest: String,
    /// PCR values (index -> hash bytes)
    pub pcrs: HashMap<usize, Vec<u8>>,
    /// Enclave certificate (DER encoded)
    pub certificate: Vec<u8>,
    /// CA bundle (list of DER encoded certificates)
    pub cabundle: Vec<Vec<u8>>,
    /// Optional user data included in attestation request
    pub user_data: Option<Vec<u8>>,
    /// Optional nonce (challenge) for freshness
    pub nonce: Option<Vec<u8>>,
    /// Optional public key
    pub public_key: Option<Vec<u8>>,
}

impl AttestationDocument {
    /// Check if this is a debug-mode attestation
    /// In debug mode, all PCR values are zeros
    pub fn is_debug_mode(&self) -> bool {
        self.pcrs.values().all(|v| v.iter().all(|&b| b == 0))
    }

    /// Get PCR value by index
    pub fn get_pcr(&self, index: usize) -> Option<&[u8]> {
        self.pcrs.get(&index).map(|v| v.as_slice())
    }

    /// Get PCR0 (Enclave image hash)
    pub fn pcr0(&self) -> Option<&[u8]> {
        self.get_pcr(pcr::IMAGE)
    }

    /// Get PCR1 (Kernel hash)
    pub fn pcr1(&self) -> Option<&[u8]> {
        self.get_pcr(pcr::KERNEL)
    }

    /// Get PCR2 (Application hash)
    pub fn pcr2(&self) -> Option<&[u8]> {
        self.get_pcr(pcr::APPLICATION)
    }
}

/// Configuration for attestation verification
#[derive(Debug, Clone)]
pub struct AttestationVerificationConfig {
    /// Expected PCR0 value (Enclave image hash)
    pub expected_pcr0: Option<Vec<u8>>,
    /// Expected PCR1 value (Kernel hash)
    pub expected_pcr1: Option<Vec<u8>>,
    /// Expected PCR2 value (Application hash)
    pub expected_pcr2: Option<Vec<u8>>,
    /// Allow debug mode attestations (PCRs all zeros)
    pub allow_debug: bool,
    /// Maximum allowed timestamp skew in seconds
    pub timestamp_skew_seconds: u64,
    /// Skip certificate chain validation
    pub skip_certificate_validation: bool,
}

impl Default for AttestationVerificationConfig {
    fn default() -> Self {
        Self {
            expected_pcr0: None,
            expected_pcr1: None,
            expected_pcr2: None,
            allow_debug: false,
            timestamp_skew_seconds: 60,
            skip_certificate_validation: false,
        }
    }
}

impl AttestationVerificationConfig {
    /// Create a configuration that allows debug mode
    pub fn debug() -> Self {
        Self {
            allow_debug: true,
            skip_certificate_validation: true,
            ..Default::default()
        }
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self, NitroError> {
        let allow_debug = std::env::var("CHINJU_NITRO_DEBUG")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if allow_debug {
            warn!("Nitro attestation verification running in DEBUG mode");
            return Ok(Self::debug());
        }

        let expected_pcr0 = std::env::var("CHINJU_NITRO_PCR0")
            .ok()
            .and_then(|s| hex::decode(&s).ok());

        let expected_pcr1 = std::env::var("CHINJU_NITRO_PCR1")
            .ok()
            .and_then(|s| hex::decode(&s).ok());

        let expected_pcr2 = std::env::var("CHINJU_NITRO_PCR2")
            .ok()
            .and_then(|s| hex::decode(&s).ok());

        let timestamp_skew_seconds: u64 = std::env::var("CHINJU_NITRO_TIMESTAMP_SKEW")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .map_err(|_| NitroError::InvalidConfig("Invalid CHINJU_NITRO_TIMESTAMP_SKEW"))?;

        Ok(Self {
            expected_pcr0,
            expected_pcr1,
            expected_pcr2,
            allow_debug: false,
            timestamp_skew_seconds,
            skip_certificate_validation: false,
        })
    }

    /// Set expected PCR0 value
    pub fn with_pcr0(mut self, pcr0: Vec<u8>) -> Self {
        self.expected_pcr0 = Some(pcr0);
        self
    }

    /// Set expected PCR1 value
    pub fn with_pcr1(mut self, pcr1: Vec<u8>) -> Self {
        self.expected_pcr1 = Some(pcr1);
        self
    }

    /// Set expected PCR2 value
    pub fn with_pcr2(mut self, pcr2: Vec<u8>) -> Self {
        self.expected_pcr2 = Some(pcr2);
        self
    }
}

/// Attestation document verifier
pub struct AttestationVerifier {
    config: AttestationVerificationConfig,
}

impl AttestationVerifier {
    /// Create a new verifier with the given configuration
    pub fn new(config: AttestationVerificationConfig) -> Self {
        Self { config }
    }

    /// Create a verifier for debug mode (skips PCR validation)
    pub fn debug_mode() -> Self {
        Self {
            config: AttestationVerificationConfig::debug(),
        }
    }

    /// Create a verifier from environment variables
    pub fn from_env() -> Result<Self, NitroError> {
        Ok(Self::new(AttestationVerificationConfig::from_env()?))
    }

    /// Verify an attestation document
    ///
    /// # Arguments
    /// * `raw_document` - Raw CBOR-encoded attestation document
    /// * `challenge` - Expected nonce/challenge for freshness verification
    ///
    /// # Returns
    /// Decoded and verified attestation document
    pub fn verify(
        &self,
        raw_document: &[u8],
        challenge: &[u8],
    ) -> Result<AttestationDocument, NitroError> {
        // Parse the attestation document
        let document = self.parse_document(raw_document)?;

        debug!(
            "Verifying attestation: module_id={}, timestamp={}, debug_mode={}",
            document.module_id,
            document.timestamp,
            document.is_debug_mode()
        );

        // Check debug mode
        if document.is_debug_mode() && !self.config.allow_debug {
            return Err(NitroError::AttestationFailed(
                "Debug mode attestation not allowed in production".to_string(),
            ));
        }

        // Verify nonce/challenge
        if let Some(nonce) = &document.nonce {
            if nonce != challenge {
                return Err(NitroError::ChallengeMismatch);
            }
        } else if !challenge.is_empty() {
            return Err(NitroError::AttestationFailed(
                "Expected nonce in attestation but none provided".to_string(),
            ));
        }

        // Verify timestamp
        self.verify_timestamp(&document)?;

        // Verify PCR values (skip if debug mode is allowed and document is debug)
        if !self.config.allow_debug || !document.is_debug_mode() {
            self.verify_pcrs(&document)?;
        }

        // Verify certificate chain (if not skipped)
        if !self.config.skip_certificate_validation {
            self.verify_certificate_chain(&document)?;
        }

        Ok(document)
    }

    /// Parse raw CBOR document into AttestationDocument
    fn parse_document(&self, raw_document: &[u8]) -> Result<AttestationDocument, NitroError> {
        // In a real implementation, this would parse the COSE Sign1 structure
        // For now, we assume the document is directly CBOR-encoded
        serde_cbor::from_slice(raw_document)
            .map_err(|e| NitroError::AttestationFailed(format!("Failed to parse document: {}", e)))
    }

    /// Verify the timestamp is within acceptable range
    fn verify_timestamp(&self, document: &AttestationDocument) -> Result<(), NitroError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| NitroError::TimestampInvalid(e.to_string()))?
            .as_millis() as u64;

        let doc_time = document.timestamp;
        let skew_ms = self.config.timestamp_skew_seconds * 1000;

        if doc_time > now + skew_ms {
            return Err(NitroError::TimestampInvalid(format!(
                "Attestation timestamp {} is in the future (now: {})",
                doc_time, now
            )));
        }

        if now > doc_time + skew_ms {
            return Err(NitroError::TimestampInvalid(format!(
                "Attestation timestamp {} is too old (now: {}, max age: {}s)",
                doc_time, now, self.config.timestamp_skew_seconds
            )));
        }

        Ok(())
    }

    /// Verify PCR values match expected values
    fn verify_pcrs(&self, document: &AttestationDocument) -> Result<(), NitroError> {
        // Verify PCR0 if expected value is set
        if let Some(expected) = &self.config.expected_pcr0 {
            let actual = document
                .pcr0()
                .ok_or_else(|| NitroError::AttestationFailed("PCR0 missing".to_string()))?;
            if actual != expected.as_slice() {
                return Err(NitroError::PcrMismatch {
                    index: pcr::IMAGE,
                    expected: hex::encode(expected),
                    actual: hex::encode(actual),
                });
            }
        }

        // Verify PCR1 if expected value is set
        if let Some(expected) = &self.config.expected_pcr1 {
            let actual = document
                .pcr1()
                .ok_or_else(|| NitroError::AttestationFailed("PCR1 missing".to_string()))?;
            if actual != expected.as_slice() {
                return Err(NitroError::PcrMismatch {
                    index: pcr::KERNEL,
                    expected: hex::encode(expected),
                    actual: hex::encode(actual),
                });
            }
        }

        // Verify PCR2 if expected value is set
        if let Some(expected) = &self.config.expected_pcr2 {
            let actual = document
                .pcr2()
                .ok_or_else(|| NitroError::AttestationFailed("PCR2 missing".to_string()))?;
            if actual != expected.as_slice() {
                return Err(NitroError::PcrMismatch {
                    index: pcr::APPLICATION,
                    expected: hex::encode(expected),
                    actual: hex::encode(actual),
                });
            }
        }

        Ok(())
    }

    /// Verify the certificate chain
    fn verify_certificate_chain(&self, document: &AttestationDocument) -> Result<(), NitroError> {
        // In a production implementation, this would:
        // 1. Parse the certificate chain from cabundle
        // 2. Verify each certificate signature
        // 3. Check the root against AWS Nitro root CA
        // 4. Verify the leaf certificate matches the document signer

        if document.certificate.is_empty() {
            return Err(NitroError::CertificateChainInvalid(
                "Missing certificate".to_string(),
            ));
        }

        if document.cabundle.is_empty() {
            return Err(NitroError::CertificateChainInvalid(
                "Missing CA bundle".to_string(),
            ));
        }

        // TODO: Implement full certificate chain validation
        // For now, we just check that the fields are present
        debug!(
            "Certificate chain validation: cert_len={}, cabundle_len={}",
            document.certificate.len(),
            document.cabundle.len()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_document(debug_mode: bool) -> AttestationDocument {
        let mut pcrs = HashMap::new();
        if debug_mode {
            pcrs.insert(0, vec![0u8; 48]);
            pcrs.insert(1, vec![0u8; 48]);
            pcrs.insert(2, vec![0u8; 48]);
        } else {
            pcrs.insert(0, vec![1u8; 48]);
            pcrs.insert(1, vec![2u8; 48]);
            pcrs.insert(2, vec![3u8; 48]);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        AttestationDocument {
            module_id: "test-module".to_string(),
            timestamp: now,
            digest: "SHA384".to_string(),
            pcrs,
            certificate: vec![1, 2, 3, 4],
            cabundle: vec![vec![5, 6, 7, 8]],
            user_data: None,
            nonce: Some(vec![9, 10, 11, 12]),
            public_key: None,
        }
    }

    #[test]
    fn test_debug_mode_detection() {
        let debug_doc = create_test_document(true);
        assert!(debug_doc.is_debug_mode());

        let prod_doc = create_test_document(false);
        assert!(!prod_doc.is_debug_mode());
    }

    #[test]
    fn test_pcr_accessors() {
        let doc = create_test_document(false);
        assert!(doc.pcr0().is_some());
        assert!(doc.pcr1().is_some());
        assert!(doc.pcr2().is_some());
        assert_eq!(doc.pcr0().unwrap(), &[1u8; 48]);
    }

    #[test]
    fn test_verification_config_default() {
        let config = AttestationVerificationConfig::default();
        assert!(!config.allow_debug);
        assert_eq!(config.timestamp_skew_seconds, 60);
    }

    #[test]
    fn test_verification_config_debug() {
        let config = AttestationVerificationConfig::debug();
        assert!(config.allow_debug);
        assert!(config.skip_certificate_validation);
    }
}
