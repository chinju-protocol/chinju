//! CHINJU Core Library
//!
//! Core implementations for the CHINJU Protocol - AI safety and governance.
//!
//! # Modules
//!
//! - `hardware`: Hardware abstraction layer (HSM, OTP, TEE, etc.)
//! - `types`: Common data types (Identifier, Timestamp, Signature, etc.)
//! - `crypto`: Cryptographic primitives
//!
//! # Features
//!
//! - `mock`: Enable mock implementations for development/testing (default)
//! - `hardware`: Enable real hardware integrations

pub mod crypto;
pub mod hardware;
pub mod types;

pub use hardware::mock;
pub use types::*;

/// CHINJU Protocol version
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// Security levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// L0: Development/Test (Mock)
    L0Development = 0,
    /// L1: Basic (Software TPM)
    L1Basic = 1,
    /// L2: Standard (TPM + TEE)
    L2Standard = 2,
    /// L3: Enterprise (HSM + TPM + TEE)
    L3Enterprise = 3,
    /// L4: Critical (HSM + QRNG + Data Diode)
    L4Critical = 4,
}
