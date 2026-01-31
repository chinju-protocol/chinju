//! Hardware abstraction traits
//!
//! These traits define the interface for hardware security modules.
//! Implementations can be real hardware or mock for testing.

use crate::types::{HardwareAttestation, Signature, TrustLevel};
use thiserror::Error;

/// Hardware-related errors
#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Slot not found: {0}")]
    SlotNotFound(String),
    #[error("Slot already burned: {0}")]
    SlotAlreadyBurned(String),
    #[error("Fuse already blown: {0}")]
    FuseAlreadyBlown(String),
    #[error("Insufficient entropy")]
    InsufficientEntropy,
    #[error("Hardware communication error: {0}")]
    CommunicationError(String),
    #[error("Attestation failed: {0}")]
    AttestationFailed(String),
    #[error("Not supported in this security level")]
    NotSupported,
    #[error("Storage full")]
    StorageFull,
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Key handle for HSM operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyHandle(pub String);

impl KeyHandle {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Slot ID for OTP storage
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SlotId(pub String);

impl SlotId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Fuse ID for eFuse operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuseId(pub String);

impl FuseId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Key algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    Ed25519,
    EcdsaP256,
    EcdsaP384,
    Rsa4096,
}

/// Trust root trait - base trait for all hardware modules
pub trait TrustRoot: Send + Sync {
    /// Check if this implementation is backed by real hardware
    fn is_hardware_backed(&self) -> bool;

    /// Get the security level
    fn security_level(&self) -> TrustLevel;

    /// Get hardware attestation
    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError>;
}

/// Hardware Security Module trait
pub trait HardwareSecurityModule: TrustRoot {
    /// Generate a new key pair
    fn generate_key_pair(
        &self,
        algorithm: KeyAlgorithm,
        label: &str,
    ) -> Result<KeyHandle, HardwareError>;

    /// Sign data with a key
    fn sign(&self, key_handle: &KeyHandle, data: &[u8]) -> Result<Signature, HardwareError>;

    /// Verify a signature
    fn verify(
        &self,
        key_handle: &KeyHandle,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, HardwareError>;

    /// Get public key bytes
    fn get_public_key(&self, key_handle: &KeyHandle) -> Result<Vec<u8>, HardwareError>;

    /// Securely erase a key
    fn secure_erase(&self, key_handle: &KeyHandle) -> Result<(), HardwareError>;

    /// Check if a key exists
    fn key_exists(&self, key_handle: &KeyHandle) -> bool;
}

/// Immutable storage trait (OTP memory)
pub trait ImmutableStorage: TrustRoot {
    /// Read data from a slot
    fn read(&self, slot: &SlotId) -> Result<Vec<u8>, HardwareError>;

    /// Burn data to a slot (one-time, irreversible)
    fn burn(&self, slot: &SlotId, data: &[u8]) -> Result<(), HardwareError>;

    /// Check if a slot is burned
    fn is_burned(&self, slot: &SlotId) -> Result<bool, HardwareError>;

    /// Get available slot count
    fn available_slots(&self) -> Result<usize, HardwareError>;

    /// Get slot size in bytes
    fn slot_size(&self) -> usize;
}

/// eFuse control trait
pub trait FuseControl: ImmutableStorage {
    /// Burn a specific fuse (irreversible)
    fn burn_fuse(&self, fuse_id: &FuseId) -> Result<(), HardwareError>;

    /// Check if a fuse is blown
    fn is_fuse_blown(&self, fuse_id: &FuseId) -> Result<bool, HardwareError>;

    /// Get total fuse count
    fn fuse_count(&self) -> usize;
}

/// Random number source trait
pub trait RandomSource: TrustRoot {
    /// Generate random bytes
    fn generate(&self, length: usize) -> Result<Vec<u8>, HardwareError>;

    /// Get entropy quality information
    fn entropy_quality(&self) -> Result<EntropyQuality, HardwareError>;

    /// Perform health check
    fn health_check(&self) -> Result<bool, HardwareError>;
}

/// Entropy quality information
#[derive(Debug, Clone)]
pub struct EntropyQuality {
    /// Bits of entropy per sample
    pub bits_per_sample: f64,
    /// Whether NIST tests passed
    pub nist_compliant: bool,
    /// Source type
    pub source_type: String,
}

/// Secure execution environment trait (TEE)
pub trait SecureExecution: TrustRoot {
    /// Execute code in secure enclave
    fn execute_secure<F, R>(&self, f: F) -> Result<R, HardwareError>
    where
        F: FnOnce() -> R + Send,
        R: Send;

    /// Seal data (encrypt with enclave key)
    fn seal_data(&self, data: &[u8]) -> Result<Vec<u8>, HardwareError>;

    /// Unseal data (decrypt with enclave key)
    fn unseal_data(&self, sealed: &[u8]) -> Result<Vec<u8>, HardwareError>;

    /// Get remote attestation report
    fn remote_attestation(&self, challenge: &[u8]) -> Result<Vec<u8>, HardwareError>;
}
