//! ZKP (Zero-Knowledge Proof) Verification Module (C12)
//!
//! Provides ZKP-based humanity verification using Groth16 proofs.
//! This module implements C12's Human Credential ZKP verification.
//!
//! ## Architecture
//!
//! ```text
//! HumanityProof (proto) → ZkpVerifier → Groth16 Verification
//!                              ↓
//!                    PreparedVerifyingKey (cached)
//! ```
//!
//! ## Feature Flags
//!
//! - `zkp`: Enables actual ZKP verification with arkworks
//! - Without `zkp`: Uses mock verification (development only)

use crate::gen::chinju::credential::HumanityProof;
use thiserror::Error;
use tracing::{info, warn};

// =============================================================================
// Error Types
// =============================================================================

/// ZKP verification errors
#[derive(Debug, Error)]
pub enum ZkpError {
    #[error("Invalid proof format: {0}")]
    InvalidProofFormat(String),

    #[error("Verification key not loaded")]
    VerifyingKeyNotLoaded,

    #[error("Proof verification failed: {0}")]
    VerificationFailed(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("ZKP feature not enabled")]
    FeatureNotEnabled,
}

// =============================================================================
// ZKP Verifier (Feature-gated implementation)
// =============================================================================

/// ZKP Verifier for human credential proofs
///
/// Uses Groth16 with BN254 curve for efficient verification.
/// The verifying key is loaded once at startup and cached.
pub struct ZkpVerifier {
    #[cfg(feature = "zkp")]
    prepared_vk: Option<ark_groth16::PreparedVerifyingKey<ark_bn254::Bn254>>,

    #[cfg(not(feature = "zkp"))]
    _phantom: std::marker::PhantomData<()>,
}

impl ZkpVerifier {
    /// Create a new ZKP verifier
    pub fn new() -> Self {
        info!("Initializing ZKP Verifier (C12)");

        #[cfg(feature = "zkp")]
        {
            Self { prepared_vk: None }
        }

        #[cfg(not(feature = "zkp"))]
        {
            warn!("ZKP feature not enabled - using mock verification");
            Self {
                _phantom: std::marker::PhantomData,
            }
        }
    }

    /// Load verifying key from bytes
    ///
    /// The verifying key should be generated during circuit setup
    /// and distributed with the application.
    #[cfg(feature = "zkp")]
    pub fn load_verifying_key(&mut self, vk_bytes: &[u8]) -> Result<(), ZkpError> {
        use ark_serialize::CanonicalDeserialize;

        let vk = ark_groth16::VerifyingKey::<ark_bn254::Bn254>::deserialize_compressed(vk_bytes)
            .map_err(|e| ZkpError::DeserializationError(e.to_string()))?;

        self.prepared_vk = Some(ark_groth16::prepare_verifying_key(&vk));

        info!("ZKP verifying key loaded successfully");
        Ok(())
    }

    /// Load verifying key (no-op when zkp feature disabled)
    #[cfg(not(feature = "zkp"))]
    pub fn load_verifying_key(&mut self, _vk_bytes: &[u8]) -> Result<(), ZkpError> {
        warn!("ZKP feature not enabled - verifying key ignored");
        Ok(())
    }

    /// Load verifying key from environment variable or file
    pub fn load_from_env(&mut self) -> Result<(), ZkpError> {
        if let Ok(vk_path) = std::env::var("CHINJU_ZKP_VERIFYING_KEY") {
            let vk_bytes = std::fs::read(&vk_path).map_err(|e| {
                ZkpError::DeserializationError(format!("Failed to read VK file: {}", e))
            })?;
            self.load_verifying_key(&vk_bytes)
        } else if let Ok(vk_hex) = std::env::var("CHINJU_ZKP_VERIFYING_KEY_HEX") {
            let vk_bytes = hex::decode(&vk_hex)
                .map_err(|e| ZkpError::DeserializationError(format!("Invalid hex: {}", e)))?;
            self.load_verifying_key(&vk_bytes)
        } else {
            warn!("No ZKP verifying key configured");
            Ok(())
        }
    }

    /// Check if verifying key is loaded
    pub fn is_ready(&self) -> bool {
        #[cfg(feature = "zkp")]
        {
            self.prepared_vk.is_some()
        }

        #[cfg(not(feature = "zkp"))]
        {
            true // Mock is always "ready"
        }
    }

    /// Verify a humanity proof
    ///
    /// ## Proof Format
    ///
    /// The proof should contain:
    /// - Groth16 proof (π_A, π_B, π_C) serialized in compressed format
    /// - Public inputs: [capability_score_hash, test_session_hash, timestamp]
    ///
    /// ## Returns
    ///
    /// - `Ok(true)` if proof is valid
    /// - `Ok(false)` if proof is invalid
    /// - `Err(ZkpError)` if verification cannot be performed
    pub fn verify_humanity_proof(&self, proof: &HumanityProof) -> Result<bool, ZkpError> {
        #[cfg(feature = "zkp")]
        {
            self.verify_with_arkworks(proof)
        }

        #[cfg(not(feature = "zkp"))]
        {
            self.verify_mock(proof)
        }
    }

    /// Actual verification using arkworks
    #[cfg(feature = "zkp")]
    fn verify_with_arkworks(&self, proof: &HumanityProof) -> Result<bool, ZkpError> {
        use ark_serialize::CanonicalDeserialize;

        let pvk = self
            .prepared_vk
            .as_ref()
            .ok_or(ZkpError::VerifyingKeyNotLoaded)?;

        // Deserialize proof
        let groth16_proof =
            ark_groth16::Proof::<ark_bn254::Bn254>::deserialize_compressed(&proof.zkp_data[..])
                .map_err(|e| ZkpError::InvalidProofFormat(e.to_string()))?;

        // Parse public inputs from proof metadata
        // Expected format: capability_score_hash (32 bytes) + test_session_hash (32 bytes) + timestamp (8 bytes)
        let public_inputs = self.parse_public_inputs(proof)?;

        // Verify the proof
        let result = ark_groth16::verify_proof(pvk, &groth16_proof, &public_inputs)
            .map_err(|e| ZkpError::VerificationFailed(e.to_string()))?;

        if result {
            info!(
                proof_type = proof.proof_type,
                "ZKP humanity proof verified successfully"
            );
        } else {
            warn!(
                proof_type = proof.proof_type,
                "ZKP humanity proof verification failed"
            );
        }

        Ok(result)
    }

    /// Parse public inputs from public_params field
    #[cfg(feature = "zkp")]
    fn parse_public_inputs(
        &self,
        proof: &HumanityProof,
    ) -> Result<Vec<ark_bn254::Fr>, ZkpError> {
        use ark_ff::PrimeField;

        // The public inputs are in the public_params field as serialized bytes
        if proof.public_params.is_empty() {
            return Err(ZkpError::InvalidProofFormat(
                "Missing public inputs in public_params".to_string(),
            ));
        }

        // Parse public_params: each field element is 32 bytes (BN254 scalar field)
        const FIELD_SIZE: usize = 32;
        if proof.public_params.len() % FIELD_SIZE != 0 {
            return Err(ZkpError::InvalidProofFormat(format!(
                "public_params length {} not divisible by {}",
                proof.public_params.len(),
                FIELD_SIZE
            )));
        }

        let mut inputs = Vec::with_capacity(proof.public_params.len() / FIELD_SIZE);
        for chunk in proof.public_params.chunks(FIELD_SIZE) {
            let field_element = ark_bn254::Fr::from_le_bytes_mod_order(chunk);
            inputs.push(field_element);
        }

        Ok(inputs)
    }

    /// Mock verification (development only)
    #[cfg(not(feature = "zkp"))]
    fn verify_mock(&self, proof: &HumanityProof) -> Result<bool, ZkpError> {
        warn!("Using MOCK ZKP verification - NOT SECURE FOR PRODUCTION");

        // Basic sanity checks
        if proof.zkp_data.is_empty() {
            return Ok(false);
        }

        // Mock: Check minimum data length (real proofs are ~192 bytes)
        if proof.zkp_data.len() < 32 {
            warn!(
                len = proof.zkp_data.len(),
                "Mock ZKP: proof data too short"
            );
            return Ok(false);
        }

        // Mock: Accept all non-empty proofs
        Ok(true)
    }
}

impl Default for ZkpVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Verify a humanity proof using the global verifier
///
/// This is a convenience function for one-off verifications.
/// For high-volume verification, create and reuse a ZkpVerifier instance.
pub fn verify_humanity_proof(proof: &HumanityProof) -> Result<bool, ZkpError> {
    let verifier = ZkpVerifier::new();
    verifier.verify_humanity_proof(proof)
}

/// Check if ZKP feature is enabled at compile time
pub const fn is_zkp_enabled() -> bool {
    cfg!(feature = "zkp")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zkp_verifier_creation() {
        let verifier = ZkpVerifier::new();
        // Without loaded key, should still work for mock
        #[cfg(not(feature = "zkp"))]
        assert!(verifier.is_ready());

        #[cfg(feature = "zkp")]
        assert!(!verifier.is_ready()); // No key loaded yet
    }

    #[test]
    fn test_mock_verification_empty_proof() {
        let verifier = ZkpVerifier::new();

        let proof = HumanityProof {
            proof_type: 1, // CAPABILITY_TEST
            zkp_data: vec![],
            public_params: vec![],
            degradation: None,
            generated_at: None,
        };

        let result = verifier.verify_humanity_proof(&proof);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Empty proof should fail
    }

    #[test]
    fn test_mock_verification_valid_proof() {
        let verifier = ZkpVerifier::new();

        let proof = HumanityProof {
            proof_type: 1, // CAPABILITY_TEST
            zkp_data: vec![0u8; 192], // Minimum realistic proof size
            public_params: vec![0u8; 64], // Two field elements
            degradation: None,
            generated_at: None,
        };

        #[cfg(not(feature = "zkp"))]
        {
            let result = verifier.verify_humanity_proof(&proof);
            assert!(result.is_ok());
            assert!(result.unwrap()); // Non-empty proof should pass in mock
        }
    }

    #[test]
    fn test_is_zkp_enabled() {
        #[cfg(feature = "zkp")]
        assert!(is_zkp_enabled());

        #[cfg(not(feature = "zkp"))]
        assert!(!is_zkp_enabled());
    }

    #[test]
    fn test_load_from_env_no_config() {
        // Clear any existing env vars
        std::env::remove_var("CHINJU_ZKP_VERIFYING_KEY");
        std::env::remove_var("CHINJU_ZKP_VERIFYING_KEY_HEX");

        let mut verifier = ZkpVerifier::new();
        let result = verifier.load_from_env();
        assert!(result.is_ok()); // Should succeed with warning
    }
}
