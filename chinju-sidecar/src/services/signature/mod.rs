//! Signature service for CHINJU Protocol
//!
//! This module provides signature operations using chinju-core hardware backends.
//! It includes type conversion utilities between chinju-core and protobuf types.
//! Also provides FROST threshold signature verification for critical operations.

mod convert;
mod service;
mod threshold;

// Note: convert module is available but re-exports are used internally
pub use service::SigningService;
pub use threshold::{ThresholdConfig, ThresholdError, ThresholdVerifier};
