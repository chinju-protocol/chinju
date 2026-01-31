//! Mock implementations for development and testing
//!
//! These implementations provide software-based alternatives to real hardware
//! for development and testing purposes. They should NOT be used in production.

mod hsm;
mod otp;
mod random;

pub use hsm::MockHsm;
pub use otp::MockOtp;
pub use random::MockRandom;

/// Warning message for mock implementations
pub const MOCK_WARNING: &str =
    "WARNING: Using mock implementation. NOT suitable for production use.";
