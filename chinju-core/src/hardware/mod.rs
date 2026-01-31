//! Hardware abstraction layer for CHINJU Protocol
//!
//! This module defines traits for hardware security modules and provides
//! implementations at different security levels:
//!
//! - L0 (Mock): Development and testing
//! - L1 (SoftHSM): Software HSM via PKCS#11
//! - L2 (TPM): TPM 2.0 integration
//! - L3 (YubiHSM): Hardware HSM

pub mod dead_mans_switch;
pub mod physical_kill_switch;
pub mod mock;
pub mod provider;
pub mod traits;

// Conditional modules based on features
#[cfg(feature = "softhsm")]
pub mod softhsm;

#[cfg(feature = "frost")]
pub mod threshold;

// TPM module is Linux-only (tss-esapi requires Linux)
#[cfg(all(feature = "tpm", target_os = "linux"))]
pub mod tpm;

// AWS Nitro Enclaves module (Linux-only)
#[cfg(all(feature = "nitro", target_os = "linux"))]
pub mod nitro;

// Re-exports
pub use dead_mans_switch::{
    DeadMansSwitch, DeadMansSwitchConfig, DeadMansSwitchError, EmergencyCallback,
    EnvironmentState, SoftDeadMansSwitch, SwitchState,
};
pub use mock::*;
pub use provider::{
    create_hsm, create_otp, create_random, HardwareConfig, HardwareProvider, HsmBackend,
    OtpBackend, RandomBackend,
};
pub use traits::*;

// Nitro Enclaves re-exports
#[cfg(all(feature = "nitro", target_os = "linux"))]
pub use nitro::{
    AttestationDocument, AttestationPolicy, AttestationVerificationConfig, AttestationVerifier,
    EnclaveRequest, EnclaveResponse, EnvelopeEncryptedData, EnvelopeEncryption, KmsClient,
    KmsConfig, NitroConfig, NitroEnclaveClient, NitroError, NitroHsm, PcrIndex, PcrValue,
    PolicyVerifier, VsockClient, VsockConfig,
};
#[cfg(all(feature = "nitro", target_os = "linux"))]
pub use provider::create_secure_execution;
