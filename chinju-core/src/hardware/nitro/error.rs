//! Nitro Enclave specific error types

use crate::hardware::traits::HardwareError;
use thiserror::Error;

/// Nitro Enclave specific errors
#[derive(Debug, Error)]
pub enum NitroError {
    /// Configuration is missing
    #[error("Configuration missing: {0}")]
    ConfigMissing(&'static str),

    /// Configuration is invalid
    #[error("Invalid configuration: {0}")]
    InvalidConfig(&'static str),

    /// Failed to connect to Enclave
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Communication error with Enclave
    #[error("Communication error: {0}")]
    CommunicationError(String),

    /// Serialization failed
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    /// Deserialization failed
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    /// Attestation verification failed
    #[error("Attestation failed: {0}")]
    AttestationFailed(String),

    /// PCR value mismatch
    #[error("PCR mismatch at index {index}: expected {expected}, got {actual}")]
    PcrMismatch {
        index: usize,
        expected: String,
        actual: String,
    },

    /// Challenge/nonce mismatch
    #[error("Challenge mismatch")]
    ChallengeMismatch,

    /// Timestamp validation failed
    #[error("Timestamp validation failed: {0}")]
    TimestampInvalid(String),

    /// Certificate chain validation failed
    #[error("Certificate chain validation failed: {0}")]
    CertificateChainInvalid(String),

    /// Enclave returned an error
    #[error("Enclave error: {code} - {message}")]
    EnclaveError { code: String, message: String },

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Timeout waiting for Enclave response
    #[error("Timeout waiting for Enclave response")]
    Timeout,

    /// Enclave not available (not running or not initialized)
    #[error("Enclave not available")]
    EnclaveNotAvailable,

    /// Operation not supported in current mode
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// KMS encryption failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// KMS decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// KMS key not found or inaccessible
    #[error("KMS key error: {0}")]
    KmsKeyError(String),
}

impl From<std::io::Error> for NitroError {
    fn from(e: std::io::Error) -> Self {
        NitroError::IoError(e.to_string())
    }
}

impl From<NitroError> for HardwareError {
    fn from(e: NitroError) -> Self {
        match e {
            NitroError::ConfigMissing(msg) => HardwareError::InvalidData(msg.to_string()),
            NitroError::InvalidConfig(msg) => HardwareError::InvalidData(msg.to_string()),
            NitroError::ConnectionFailed(msg) => HardwareError::CommunicationError(msg),
            NitroError::CommunicationError(msg) => HardwareError::CommunicationError(msg),
            NitroError::AttestationFailed(msg) => HardwareError::AttestationFailed(msg),
            NitroError::PcrMismatch {
                index,
                expected,
                actual,
            } => HardwareError::AttestationFailed(format!(
                "PCR{} mismatch: expected {}, got {}",
                index, expected, actual
            )),
            NitroError::ChallengeMismatch => {
                HardwareError::AttestationFailed("Challenge mismatch".to_string())
            }
            NitroError::TimestampInvalid(msg) => HardwareError::AttestationFailed(msg),
            NitroError::CertificateChainInvalid(msg) => HardwareError::AttestationFailed(msg),
            NitroError::EnclaveError { code, message } => {
                HardwareError::CommunicationError(format!("{}: {}", code, message))
            }
            NitroError::IoError(msg) => HardwareError::CommunicationError(msg),
            NitroError::Timeout => {
                HardwareError::CommunicationError("Timeout waiting for Enclave".to_string())
            }
            NitroError::EnclaveNotAvailable => {
                HardwareError::CommunicationError("Enclave not available".to_string())
            }
            NitroError::NotSupported(_) => HardwareError::NotSupported,
            NitroError::EncryptionFailed(msg) => HardwareError::CommunicationError(msg),
            NitroError::DecryptionFailed(msg) => HardwareError::CommunicationError(msg),
            NitroError::KmsKeyError(msg) => HardwareError::KeyNotFound(msg),
            _ => HardwareError::CommunicationError(e.to_string()),
        }
    }
}
