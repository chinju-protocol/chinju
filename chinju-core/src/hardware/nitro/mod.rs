//! AWS Nitro Enclaves integration module
//!
//! This module provides L3 (Enterprise) level secure execution
//! using AWS Nitro Enclaves for model containment (C13).
//!
//! # Architecture
//!
//! ```text
//! EC2 Parent Instance
//! +--------------------------------------------------+
//! |  chinju-sidecar                                  |
//! |    NitroEnclaveClient <-- vsock --> Enclave      |
//! +--------------------------------------------------+
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use chinju_core::hardware::nitro::{NitroHsm, NitroConfig};
//!
//! let config = NitroConfig::from_env()?;
//! let hsm = NitroHsm::new(config)?;
//!
//! // Get attestation document
//! let attestation = hsm.remote_attestation(&challenge)?;
//!
//! // Seal data
//! let sealed = hsm.seal_data(&plaintext)?;
//! ```

mod attestation;
mod attestation_policy;
mod client;
mod error;
mod hsm;
mod kms;
mod protocol;
mod vsock;

pub use attestation::{AttestationDocument, AttestationVerificationConfig, AttestationVerifier};
pub use attestation_policy::{AttestationPolicy, PcrIndex, PcrValue, PolicyVerifier};
pub use client::NitroEnclaveClient;
pub use error::NitroError;
pub use hsm::{NitroConfig, NitroHsm};
pub use kms::{EnvelopeEncryptedData, EnvelopeEncryption, KmsClient, KmsConfig};
pub use protocol::{EnclaveRequest, EnclaveResponse};
pub use vsock::{VsockClient, VsockConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Ensure all public types are accessible
        let _config = VsockConfig {
            cid: 16,
            port: 5000,
            timeout_ms: 5000,
        };
    }
}
