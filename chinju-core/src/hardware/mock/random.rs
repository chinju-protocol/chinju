//! Mock random number generator
//!
//! Uses system PRNG for development and testing.
//! WARNING: NOT quantum random - NOT suitable for production use!

use crate::hardware::traits::*;
use crate::types::{HardwareAttestation, Timestamp, TrustLevel};
use rand::RngCore;
use tracing::warn;

/// Mock random number generator
///
/// Uses the system's cryptographically secure PRNG.
/// For production, use real QRNG (Quantum Random Number Generator).
pub struct MockRandom {
    /// Whether to simulate failures for testing
    fail_rate: f64,
}

impl MockRandom {
    /// Create a new mock random source
    pub fn new() -> Self {
        warn!("{}", super::MOCK_WARNING);
        Self { fail_rate: 0.0 }
    }

    /// Create with simulated failure rate (0.0 - 1.0)
    pub fn with_fail_rate(fail_rate: f64) -> Self {
        Self { fail_rate }
    }
}

impl Default for MockRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustRoot for MockRandom {
    fn is_hardware_backed(&self) -> bool {
        false
    }

    fn security_level(&self) -> TrustLevel {
        TrustLevel::Mock
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        Ok(HardwareAttestation {
            trust_level: TrustLevel::Mock,
            hardware_type: "MockRandom (PRNG)".to_string(),
            attestation_data: b"MOCK_RANDOM_NOT_QUANTUM".to_vec(),
            manufacturer_signature: None,
            attested_at: Timestamp::now(),
        })
    }
}

impl RandomSource for MockRandom {
    fn generate(&self, length: usize) -> Result<Vec<u8>, HardwareError> {
        // Simulate random failures for testing
        if self.fail_rate > 0.0 {
            let mut rng = rand::thread_rng();
            let roll: f64 = rng.gen();
            if roll < self.fail_rate {
                return Err(HardwareError::InsufficientEntropy);
            }
        }

        let mut bytes = vec![0u8; length];
        rand::thread_rng().fill_bytes(&mut bytes);
        Ok(bytes)
    }

    fn entropy_quality(&self) -> Result<EntropyQuality, HardwareError> {
        Ok(EntropyQuality {
            bits_per_sample: 1.0, // Assumed good for CSPRNG
            nist_compliant: false, // Not tested
            source_type: "CSPRNG (rand crate)".to_string(),
        })
    }

    fn health_check(&self) -> Result<bool, HardwareError> {
        // Simple health check: can we generate random bytes?
        match self.generate(32) {
            Ok(bytes) => {
                // Check that not all zeros (extremely unlikely with good RNG)
                Ok(bytes.iter().any(|&b| b != 0))
            }
            Err(_) => Ok(false),
        }
    }
}

// Need to add this trait bound usage
use rand::Rng;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_random_generate() {
        let rng = MockRandom::new();

        let bytes1 = rng.generate(32).unwrap();
        let bytes2 = rng.generate(32).unwrap();

        // Should be 32 bytes
        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);

        // Should be different (extremely high probability)
        assert_ne!(bytes1, bytes2);
    }

    #[test]
    fn test_mock_random_health_check() {
        let rng = MockRandom::new();
        assert!(rng.health_check().unwrap());
    }

    #[test]
    fn test_mock_random_entropy_quality() {
        let rng = MockRandom::new();
        let quality = rng.entropy_quality().unwrap();

        assert!(quality.bits_per_sample > 0.0);
        assert_eq!(quality.source_type, "CSPRNG (rand crate)");
    }

    #[test]
    fn test_mock_random_attestation() {
        let rng = MockRandom::new();
        let attestation = rng.get_attestation().unwrap();

        assert_eq!(attestation.trust_level, TrustLevel::Mock);
        assert!(!rng.is_hardware_backed());
    }
}
