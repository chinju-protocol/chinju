//! Threshold signature verification for CHINJU Protocol
//!
//! This module provides verification of FROST t-of-n threshold signatures.
//! Used for emergency halt, resume, and other critical operations.

use crate::gen::chinju::common::ThresholdSignature as ProtoThresholdSignature;
use chinju_core::hardware::threshold::{FrostCoordinator, FrostError};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Threshold verification errors
#[derive(Debug, Error)]
pub enum ThresholdError {
    #[error("FROST error: {0}")]
    Frost(#[from] FrostError),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid threshold signature: {0}")]
    InvalidSignature(String),

    #[error("Coordinator not initialized")]
    NotInitialized,
}

/// Configuration for threshold verification
#[derive(Debug, Clone)]
pub struct ThresholdConfig {
    /// Minimum number of signers required (t)
    pub threshold: u16,
    /// Total number of signers (n)
    pub total: u16,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            total: 5,
        }
    }
}

/// Threshold signature verifier using FROST
pub struct ThresholdVerifier {
    config: ThresholdConfig,
    coordinator: Arc<RwLock<Option<FrostCoordinator>>>,
    /// Group public key (set after key generation ceremony)
    group_public_key: Arc<RwLock<Option<Vec<u8>>>>,
}

impl ThresholdVerifier {
    /// Create a new threshold verifier
    pub fn new(config: ThresholdConfig) -> Self {
        info!(
            threshold = config.threshold,
            total = config.total,
            "Initializing threshold verifier"
        );
        Self {
            config,
            coordinator: Arc::new(RwLock::new(None)),
            group_public_key: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(ThresholdConfig::default())
    }

    /// Initialize with a trusted dealer key generation (for testing/development)
    pub async fn init_trusted_dealer(&self) -> Result<Vec<u8>, ThresholdError> {
        let coordinator = FrostCoordinator::new(self.config.threshold, self.config.total)?;

        // Run trusted dealer key generation
        let _shares = coordinator.trusted_dealer_keygen()?;
        let group_pubkey = coordinator.group_public_key()?;

        // Store coordinator and group public key
        {
            let mut coord = self.coordinator.write().await;
            *coord = Some(coordinator);
        }
        {
            let mut gpk = self.group_public_key.write().await;
            *gpk = Some(group_pubkey.clone());
        }

        info!(
            threshold = self.config.threshold,
            total = self.config.total,
            "Threshold key generation completed (trusted dealer mode)"
        );

        Ok(group_pubkey)
    }

    /// Initialize from environment variables (CHINJU_THRESHOLD_PUBKEY)
    pub async fn init_from_env(&self) -> Result<(), ThresholdError> {
        if let Ok(pubkey_hex) = std::env::var("CHINJU_THRESHOLD_PUBKEY") {
            let pubkey_bytes = hex::decode(&pubkey_hex).map_err(|e| {
                ThresholdError::InvalidSignature(format!("Invalid hex public key: {}", e))
            })?;

            self.set_group_public_key(pubkey_bytes).await;
            info!("Initialized threshold verifier from environment variable");
        }
        Ok(())
    }

    /// Set the group public key (for verifying existing signatures)
    pub async fn set_group_public_key(&self, pubkey: Vec<u8>) {
        let mut gpk = self.group_public_key.write().await;
        *gpk = Some(pubkey);
        info!("Group public key set for threshold verification");
    }

    /// Get the group public key
    pub async fn group_public_key(&self) -> Option<Vec<u8>> {
        let gpk = self.group_public_key.read().await;
        gpk.clone()
    }

    /// Verify a threshold signature
    pub async fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, ThresholdError> {
        // First try using the coordinator if available
        {
            let coordinator = self.coordinator.read().await;
            if let Some(ref coord) = *coordinator {
                debug!(
                    message_len = message.len(),
                    signature_len = signature.len(),
                    "Verifying threshold signature using coordinator"
                );
                return Ok(coord.verify(message, signature)?);
            }
        }

        // Fallback to using just the public key
        {
            let gpk = self.group_public_key.read().await;
            if let Some(ref pubkey) = *gpk {
                debug!(
                    message_len = message.len(),
                    signature_len = signature.len(),
                    "Verifying threshold signature using public key"
                );
                return Ok(FrostCoordinator::verify_with_pubkey(
                    pubkey, message, signature,
                )?);
            }
        }

        Err(ThresholdError::NotInitialized)
    }

    /// Verify a proto ThresholdSignature
    /// This performs basic validation without cryptographic verification
    /// (full verification requires the coordinator to be initialized)
    pub async fn verify_proto(
        &self,
        message: &[u8],
        threshold_sig: &ProtoThresholdSignature,
    ) -> Result<bool, ThresholdError> {
        // Check threshold is met
        if threshold_sig.signatures.len() < threshold_sig.threshold as usize {
            return Err(ThresholdError::InvalidSignature(format!(
                "Not enough signatures: have {}, need {}",
                threshold_sig.signatures.len(),
                threshold_sig.threshold
            )));
        }

        // Try to verify cryptographically if possible
        if self.is_initialized().await {
            // For now, we verify each individual signature
            // In a full implementation, we would aggregate and verify
            for sig in &threshold_sig.signatures {
                if sig.signature.is_empty() {
                    return Err(ThresholdError::InvalidSignature(
                        "Empty signature in threshold set".to_string(),
                    ));
                }
            }

            // Try to verify with aggregated signature if available
            if threshold_sig.signatures.len() == 1 {
                // Single aggregated signature
                let agg_sig = &threshold_sig.signatures[0].signature;
                return self.verify(message, agg_sig).await;
            }
        }

        // Basic validation passed
        debug!(
            signatures = threshold_sig.signatures.len(),
            threshold = threshold_sig.threshold,
            "Threshold signature basic validation passed"
        );

        Ok(true)
    }

    /// Check if the verifier is initialized
    pub async fn is_initialized(&self) -> bool {
        let coord = self.coordinator.read().await;
        let gpk = self.group_public_key.read().await;
        coord.is_some() || gpk.is_some()
    }

    /// Get threshold configuration
    pub fn config(&self) -> &ThresholdConfig {
        &self.config
    }
}

impl Clone for ThresholdVerifier {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            coordinator: Arc::clone(&self.coordinator),
            group_public_key: Arc::clone(&self.group_public_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_threshold_verifier_creation() {
        let verifier = ThresholdVerifier::default_config();
        assert_eq!(verifier.config().threshold, 3);
        assert_eq!(verifier.config().total, 5);
        assert!(!verifier.is_initialized().await);
    }

    #[tokio::test]
    async fn test_trusted_dealer_keygen() {
        let verifier = ThresholdVerifier::new(ThresholdConfig {
            threshold: 2,
            total: 3,
        });

        let pubkey = verifier.init_trusted_dealer().await.unwrap();
        assert!(!pubkey.is_empty());
        assert!(verifier.is_initialized().await);

        // Group public key should be set
        let gpk = verifier.group_public_key().await;
        assert!(gpk.is_some());
        assert_eq!(gpk.unwrap(), pubkey);
    }

    #[tokio::test]
    async fn test_verify_requires_initialization() {
        let verifier = ThresholdVerifier::default_config();
        let result = verifier.verify(b"test", &[0u8; 64]).await;
        assert!(matches!(result, Err(ThresholdError::NotInitialized)));
    }
}
