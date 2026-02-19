//! Signing service implementation

use crate::gen::chinju::common as proto;
use chinju_core::hardware::{
    HardwareConfig, HardwareError, HardwareProvider, KeyAlgorithm, KeyHandle,
};
use chinju_core::types as core;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Signing service errors
#[derive(Debug, Error)]
pub enum SigningError {
    #[error("Hardware error: {0}")]
    Hardware(#[from] HardwareError),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

/// Key metadata
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_id: String,
    pub algorithm: KeyAlgorithm,
    pub handle: KeyHandle,
    pub created_at: core::Timestamp,
}

/// Signing service that wraps HardwareProvider
pub struct SigningService {
    provider: Arc<HardwareProvider>,
    /// Cached key handles (key_id -> metadata)
    keys: Arc<RwLock<std::collections::HashMap<String, KeyMetadata>>>,
    /// Default issuer key ID
    issuer_key_id: String,
}

impl SigningService {
    /// Create a new signing service from hardware configuration
    pub fn new(config: HardwareConfig) -> Result<Self, SigningError> {
        let provider = HardwareProvider::new(config)?;
        Ok(Self {
            provider: Arc::new(provider),
            keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
            issuer_key_id: String::new(),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, SigningError> {
        let config = HardwareConfig::from_env()?;
        Self::new(config)
    }

    /// Create with default mock configuration (for testing)
    pub fn mock() -> Self {
        let config = HardwareConfig::default();
        Self::new(config).expect("Mock config should always succeed")
    }

    /// Get the trust level of the underlying hardware
    pub fn trust_level(&self) -> core::TrustLevel {
        self.provider.hsm().security_level()
    }

    /// Get trust level as proto type
    pub fn trust_level_proto(&self) -> proto::TrustLevel {
        proto::TrustLevel::from(self.trust_level())
    }

    /// Generate a new signing key pair
    pub async fn generate_key(
        &self,
        key_id: &str,
        algorithm: KeyAlgorithm,
    ) -> Result<KeyMetadata, SigningError> {
        info!(key_id = %key_id, algorithm = ?algorithm, "Generating signing key");

        let handle = self.provider.hsm().generate_key_pair(algorithm, key_id)?;

        let metadata = KeyMetadata {
            key_id: key_id.to_string(),
            algorithm,
            handle,
            created_at: core::Timestamp::now(),
        };

        // Cache the key
        {
            let mut keys = self.keys.write().await;
            keys.insert(key_id.to_string(), metadata.clone());
        }

        Ok(metadata)
    }

    /// Generate or get the default issuer key
    pub async fn ensure_issuer_key(&mut self) -> Result<String, SigningError> {
        if !self.issuer_key_id.is_empty() {
            return Ok(self.issuer_key_id.clone());
        }

        let key_id = format!("chinju-issuer-{}", uuid::Uuid::new_v4());
        self.generate_key(&key_id, KeyAlgorithm::Ed25519).await?;
        self.issuer_key_id = key_id.clone();

        info!(key_id = %key_id, "Generated default issuer key");
        Ok(key_id)
    }

    /// Get a key by ID
    pub async fn get_key(&self, key_id: &str) -> Option<KeyMetadata> {
        let keys = self.keys.read().await;
        keys.get(key_id).cloned()
    }

    /// Sign data with a specific key
    pub async fn sign(&self, key_id: &str, data: &[u8]) -> Result<core::Signature, SigningError> {
        let metadata = {
            let keys = self.keys.read().await;
            keys.get(key_id)
                .cloned()
                .ok_or_else(|| SigningError::KeyNotFound(key_id.to_string()))?
        };

        debug!(key_id = %key_id, data_len = data.len(), "Signing data");

        let sig = self.provider.hsm().sign(&metadata.handle, data)?;

        Ok(sig)
    }

    /// Sign data and return proto::Signature
    pub async fn sign_proto(
        &self,
        key_id: &str,
        data: &[u8],
    ) -> Result<proto::Signature, SigningError> {
        let sig = self.sign(key_id, data).await?;
        Ok(proto::Signature::from(sig))
    }

    /// Sign with the default issuer key (for credential issuance)
    pub async fn sign_as_issuer(&self, data: &[u8]) -> Result<proto::Signature, SigningError> {
        if self.issuer_key_id.is_empty() {
            return Err(SigningError::KeyNotFound(
                "Issuer key not initialized".to_string(),
            ));
        }
        self.sign_proto(&self.issuer_key_id, data).await
    }

    /// Get issuer key ID
    pub fn issuer_key_id(&self) -> &str {
        &self.issuer_key_id
    }

    /// Get issuer public key
    pub async fn issuer_public_key(&self) -> Result<Vec<u8>, SigningError> {
        if self.issuer_key_id.is_empty() {
            return Err(SigningError::KeyNotFound(
                "Issuer key not initialized".to_string(),
            ));
        }

        let metadata = {
            let keys = self.keys.read().await;
            keys.get(&self.issuer_key_id)
                .cloned()
                .ok_or_else(|| SigningError::KeyNotFound(self.issuer_key_id.clone()))?
        };

        let pubkey = self.provider.hsm().get_public_key(&metadata.handle)?;
        Ok(pubkey)
    }

    /// Verify a signature
    pub async fn verify(
        &self,
        key_id: &str,
        data: &[u8],
        signature: &core::Signature,
    ) -> Result<bool, SigningError> {
        let metadata = {
            let keys = self.keys.read().await;
            keys.get(key_id)
                .cloned()
                .ok_or_else(|| SigningError::KeyNotFound(key_id.to_string()))?
        };

        debug!(key_id = %key_id, data_len = data.len(), "Verifying signature");

        let valid = self
            .provider
            .hsm()
            .verify(&metadata.handle, data, signature)?;

        Ok(valid)
    }

    /// Verify a proto::Signature
    pub async fn verify_proto(
        &self,
        key_id: &str,
        data: &[u8],
        signature: &proto::Signature,
    ) -> Result<bool, SigningError> {
        let core_sig = core::Signature::from(signature.clone());
        self.verify(key_id, data, &core_sig).await
    }

    /// Get hardware attestation
    pub fn get_attestation(&self) -> Result<core::HardwareAttestation, SigningError> {
        let attestation = self.provider.hsm().get_attestation()?;
        Ok(attestation)
    }

    /// Get hardware attestation as proto type
    pub fn get_attestation_proto(&self) -> Result<proto::HardwareAttestation, SigningError> {
        let att = self.get_attestation()?;

        Ok(proto::HardwareAttestation {
            trust_level: proto::TrustLevel::from(att.trust_level).into(),
            hardware_type: att.hardware_type,
            attestation_data: att.attestation_data,
            manufacturer_signature: att.manufacturer_signature.map(proto::Signature::from),
            attested_at: Some(proto::Timestamp::from(&att.attested_at)),
        })
    }
}

impl Clone for SigningService {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
            keys: Arc::clone(&self.keys),
            issuer_key_id: self.issuer_key_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signing_service_mock() {
        let mut service = SigningService::mock();

        // Should be mock trust level
        assert_eq!(service.trust_level(), core::TrustLevel::Mock);

        // Generate issuer key
        let key_id = service.ensure_issuer_key().await.unwrap();
        assert!(!key_id.is_empty());

        // Sign data
        let data = b"test data";
        let sig = service.sign_as_issuer(data).await.unwrap();
        assert!(!sig.signature.is_empty());
        assert!(!sig.public_key.is_empty());
    }

    #[tokio::test]
    async fn test_key_generation() {
        let service = SigningService::mock();

        let metadata = service
            .generate_key("test-key", KeyAlgorithm::Ed25519)
            .await
            .unwrap();

        assert_eq!(metadata.key_id, "test-key");
        assert_eq!(metadata.algorithm, KeyAlgorithm::Ed25519);

        // Should be cached
        let cached = service.get_key("test-key").await;
        assert!(cached.is_some());
    }
}
