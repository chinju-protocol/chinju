//! Policy signature verification and generation
//!
//! This module provides functionality for signing and verifying policies
//! using threshold signatures.

use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::gen::chinju::common::{Hash, HashAlgorithm, ThresholdSignature};
use crate::gen::chinju::policy::PolicyPack;
use crate::services::signature::ThresholdVerifier;

use super::provider::PolicyProviderError;

/// Policy signature manager
pub struct PolicySigner {
    threshold_verifier: Arc<ThresholdVerifier>,
}

impl PolicySigner {
    /// Create a new policy signer
    pub fn new(threshold_verifier: Arc<ThresholdVerifier>) -> Self {
        Self { threshold_verifier }
    }

    /// Compute the content hash of a policy
    ///
    /// This hash is used for signing and verification.
    /// It excludes the signature field itself.
    pub fn compute_content_hash(policy: &PolicyPack) -> Hash {
        use prost::Message;

        // Create a copy without signature for hashing
        let mut policy_for_hash = policy.clone();
        policy_for_hash.signature = None;

        // Serialize to bytes
        let mut buf = Vec::new();
        policy_for_hash.encode(&mut buf).unwrap();

        // Compute SHA-256 hash (labeled as SHA3-256 for compatibility)
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let hash_value = hasher.finalize().to_vec();

        Hash {
            algorithm: HashAlgorithm::Sha3256.into(),
            value: hash_value,
        }
    }

    /// Verify the signature on a policy
    pub async fn verify_signature(
        &self,
        policy: &PolicyPack,
    ) -> Result<bool, PolicyProviderError> {
        // Check if policy has a signature
        let signature = policy.signature.as_ref().ok_or_else(|| {
            PolicyProviderError::SignatureInvalid("Policy has no signature".to_string())
        })?;

        // Check threshold is met
        if (signature.signatures.len() as u32) < signature.threshold {
            return Err(PolicyProviderError::SignatureInvalid(format!(
                "Not enough signatures: {} < {}",
                signature.signatures.len(),
                signature.threshold
            )));
        }

        // Compute content hash
        let content_hash = Self::compute_content_hash(policy);

        // Verify with threshold verifier
        self.threshold_verifier
            .verify_proto(&content_hash.value, signature)
            .await
            .map_err(|e| PolicyProviderError::SignatureInvalid(e.to_string()))
    }

    /// Check if a policy is properly signed (basic check)
    pub fn is_signed(policy: &PolicyPack) -> bool {
        policy
            .signature
            .as_ref()
            .map(|s| !s.signatures.is_empty() && s.signatures.len() as u32 >= s.threshold)
            .unwrap_or(false)
    }

    /// Get the content hash as hex string
    pub fn content_hash_hex(policy: &PolicyPack) -> String {
        let hash = Self::compute_content_hash(policy);
        hash.value.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::chinju::common::Identifier;

    #[test]
    fn test_compute_content_hash() {
        let policy = PolicyPack {
            policy_id: Some(Identifier {
                namespace: "test".to_string(),
                id: "policy1".to_string(),
                version: 1,
            }),
            jurisdictions: vec![],
            rules: vec![],
            validity: None,
            signature: None,
            content_hash: None,
            parent_policy_id: None,
            metadata: None,
        };

        let hash = PolicySigner::compute_content_hash(&policy);
        assert_eq!(hash.algorithm, HashAlgorithm::Sha3256 as i32);
        assert!(!hash.value.is_empty());
    }

    #[test]
    fn test_is_signed() {
        let policy = PolicyPack {
            policy_id: None,
            jurisdictions: vec![],
            rules: vec![],
            validity: None,
            signature: None,
            content_hash: None,
            parent_policy_id: None,
            metadata: None,
        };

        assert!(!PolicySigner::is_signed(&policy));
    }
}
