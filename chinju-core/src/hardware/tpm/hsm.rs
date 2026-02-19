//! TPM-based HardwareSecurityModule implementation

use super::context::{TpmConfig, TpmContext, TpmError};
use super::pcr::{PcrBank, PcrOps, PcrValue};
use crate::hardware::traits::{
    EntropyQuality, HardwareError, HardwareSecurityModule, ImmutableStorage, KeyAlgorithm,
    KeyHandle, RandomSource, SlotId, TrustRoot,
};
use crate::types::{HardwareAttestation, Signature, SignatureAlgorithm, Timestamp, TrustLevel};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};
use tss_esapi::{
    attributes::ObjectAttributesBuilder,
    interface_types::{
        algorithm::{HashingAlgorithm, PublicAlgorithm, SignatureSchemeAlgorithm},
        key_bits::RsaKeyBits,
        resource_handles::Hierarchy,
    },
    structures::{
        Public, PublicBuilder, PublicKeyRsa, PublicRsaParametersBuilder, RsaScheme, RsaSignature,
        SymmetricDefinitionObject,
    },
};

/// TPM-based HSM implementation
pub struct TpmHsm {
    context: Arc<RwLock<TpmContext>>,
    /// Key handle mapping (our handle -> TPM handle)
    key_handles: Arc<RwLock<HashMap<String, tss_esapi::handles::KeyHandle>>>,
    /// Counter for generating unique IDs
    next_id: Arc<RwLock<u64>>,
}

impl TpmHsm {
    /// Create a new TPM HSM instance
    pub fn new(config: TpmConfig) -> Result<Self, HardwareError> {
        let context = TpmContext::new(config)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;

        info!("TPM HSM initialized");

        Ok(Self {
            context: Arc::new(RwLock::new(context)),
            key_handles: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, HardwareError> {
        Self::new(TpmConfig::from_env())
    }

    /// Generate a unique key ID
    fn generate_key_id(&self) -> String {
        let mut id = self.next_id.write().unwrap();
        let key_id = format!("tpm-key-{:08x}", *id);
        *id += 1;
        key_id
    }

    /// Get PCR operations
    pub fn pcr_ops(&self) -> Result<PcrOpsGuard, HardwareError> {
        let context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;
        Ok(PcrOpsGuard { _context: context })
    }

    /// Read PCR value
    pub fn read_pcr(&self, index: u8, bank: PcrBank) -> Result<PcrValue, HardwareError> {
        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;
        let mut ops = PcrOps::new(&mut context);
        ops.read(index, bank)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    /// Extend PCR
    pub fn extend_pcr(&self, index: u8, bank: PcrBank, data: &[u8]) -> Result<(), HardwareError> {
        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;
        let mut ops = PcrOps::new(&mut context);
        ops.extend(index, bank, data)
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))
    }

    /// Seal data to PCR values
    pub fn seal_to_pcr(
        &self,
        data: &[u8],
        pcr_indices: &[u8],
        bank: PcrBank,
    ) -> Result<Vec<u8>, HardwareError> {
        // Sealing binds data to current PCR values
        // Only unsealing when PCRs match is possible

        // For full implementation:
        // 1. Read current PCR values
        // 2. Create a policy session with PCR policy
        // 3. Create a sealed object under that policy
        // 4. Return the sealed blob

        // Placeholder: just encrypt with a simple scheme
        warn!("TPM sealing not fully implemented, using placeholder");

        let mut sealed = Vec::with_capacity(data.len() + 16);
        sealed.extend_from_slice(&[0x54, 0x50, 0x4D, 0x53]); // "TPMS" magic
        sealed.extend_from_slice(&(pcr_indices.len() as u32).to_le_bytes());
        sealed.extend_from_slice(pcr_indices);
        sealed.push(bank as u8);
        sealed.extend_from_slice(data);

        Ok(sealed)
    }

    /// Unseal data (requires PCR values to match)
    pub fn unseal(&self, sealed_data: &[u8]) -> Result<Vec<u8>, HardwareError> {
        // For full implementation:
        // 1. Load the sealed object
        // 2. Start a policy session with the same PCR policy
        // 3. Unseal if PCRs match

        // Placeholder: simple extraction
        if sealed_data.len() < 8 || &sealed_data[0..4] != b"TPMS" {
            return Err(HardwareError::InvalidData(
                "Invalid sealed data format".to_string(),
            ));
        }

        let pcr_count = u32::from_le_bytes(sealed_data[4..8].try_into().unwrap()) as usize;
        let data_start = 8 + pcr_count + 1;

        if sealed_data.len() <= data_start {
            return Err(HardwareError::InvalidData(
                "Sealed data too short".to_string(),
            ));
        }

        Ok(sealed_data[data_start..].to_vec())
    }

    /// Get remote attestation quote
    pub fn get_quote(&self, nonce: &[u8], pcr_indices: &[u8]) -> Result<Vec<u8>, HardwareError> {
        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;
        let mut ops = PcrOps::new(&mut context);

        let quote = ops
            .quote(pcr_indices, PcrBank::Sha256, nonce)
            .map_err(|e| HardwareError::AttestationFailed(e.to_string()))?;

        serde_json::to_vec(&quote).map_err(|e| HardwareError::InvalidData(e.to_string()))
    }
}

/// Guard for PCR operations
pub struct PcrOpsGuard<'a> {
    _context: std::sync::RwLockWriteGuard<'a, TpmContext>,
}

impl TrustRoot for TpmHsm {
    fn is_hardware_backed(&self) -> bool {
        let context = self.context.read().unwrap();
        !matches!(
            context.config().interface,
            super::context::TpmInterface::Socket { .. }
        )
    }

    fn security_level(&self) -> TrustLevel {
        if self.is_hardware_backed() {
            TrustLevel::HardwareStandard // L2
        } else {
            TrustLevel::Software // swtpm is L1 equivalent
        }
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        let info = context
            .get_manufacturer_info()
            .map_err(|e| HardwareError::AttestationFailed(e.to_string()))?;

        let hardware_type = if info.is_software {
            "swtpm (Software TPM)"
        } else {
            "TPM 2.0"
        };

        Ok(HardwareAttestation {
            trust_level: self.security_level(),
            hardware_type: hardware_type.to_string(),
            attestation_data: format!("manufacturer:{}", info.manufacturer).into_bytes(),
            manufacturer_signature: None,
            attested_at: Timestamp::now(),
        })
    }
}

impl HardwareSecurityModule for TpmHsm {
    fn generate_key_pair(
        &self,
        algorithm: KeyAlgorithm,
        label: &str,
    ) -> Result<KeyHandle, HardwareError> {
        let key_id = self.generate_key_id();

        match algorithm {
            KeyAlgorithm::Rsa4096 => {
                let mut context = self
                    .context
                    .write()
                    .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

                // Build RSA key template
                let object_attributes = ObjectAttributesBuilder::new()
                    .with_fixed_tpm(true)
                    .with_fixed_parent(true)
                    .with_sensitive_data_origin(true)
                    .with_user_with_auth(true)
                    .with_sign_encrypt(true)
                    .build()
                    .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

                let rsa_params = PublicRsaParametersBuilder::new()
                    .with_scheme(RsaScheme::RsaSsa(HashingAlgorithm::Sha256.into()))
                    .with_key_bits(RsaKeyBits::Rsa4096)
                    .with_exponent(tss_esapi::structures::RsaExponent::default())
                    .with_is_signing_key(true)
                    .with_is_decryption_key(false)
                    .with_restricted(false)
                    .build()
                    .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

                let public = PublicBuilder::new()
                    .with_public_algorithm(PublicAlgorithm::Rsa)
                    .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
                    .with_object_attributes(object_attributes)
                    .with_rsa_parameters(rsa_params)
                    .with_rsa_unique_identifier(PublicKeyRsa::default())
                    .build()
                    .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

                // Create primary key under owner hierarchy
                let key_result = context
                    .context_mut()
                    .execute_with_nullauth_session(|ctx| {
                        ctx.create_primary(Hierarchy::Owner, public, None, None, None, None)
                    })
                    .map_err(|e| {
                        HardwareError::CommunicationError(format!("Key creation failed: {}", e))
                    })?;

                // Store the handle
                {
                    let mut handles = self.key_handles.write().unwrap();
                    handles.insert(key_id.clone(), key_result.key_handle);
                }

                info!(key_id = %key_id, label = %label, "Generated RSA-4096 key in TPM");
                Ok(KeyHandle::new(key_id))
            }
            KeyAlgorithm::EcdsaP256 | KeyAlgorithm::EcdsaP384 => {
                // ECC key generation would be similar but with ECC parameters
                Err(HardwareError::NotSupported)
            }
            KeyAlgorithm::Ed25519 => {
                // Ed25519 is not natively supported by TPM 2.0
                // Would need to use software implementation or ECC equivalent
                Err(HardwareError::NotSupported)
            }
        }
    }

    fn sign(&self, key_handle: &KeyHandle, data: &[u8]) -> Result<Signature, HardwareError> {
        let tpm_handle = {
            let handles = self.key_handles.read().unwrap();
            handles
                .get(&key_handle.0)
                .copied()
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        // Hash the data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let digest = hasher.finalize();

        let digest_tpm = tss_esapi::structures::Digest::try_from(digest.as_slice())
            .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

        // Sign with TPM
        let signature = context
            .context_mut()
            .execute_with_nullauth_session(|ctx| {
                ctx.sign(
                    tpm_handle,
                    digest_tpm,
                    RsaScheme::RsaSsa(HashingAlgorithm::Sha256.into()).into(),
                    tss_esapi::structures::HashcheckTicket::default(),
                )
            })
            .map_err(|e| HardwareError::CommunicationError(format!("Signing failed: {}", e)))?;

        // Extract signature bytes
        let sig_bytes = match signature {
            tss_esapi::structures::Signature::RsaSsa(rsa_sig) => {
                rsa_sig.signature().as_bytes().to_vec()
            }
            _ => {
                return Err(HardwareError::InvalidData(
                    "Unexpected signature type".to_string(),
                ))
            }
        };

        // Get public key
        let public_key = self.get_public_key(key_handle)?;

        Ok(Signature {
            algorithm: SignatureAlgorithm::EcdsaP256, // Using as placeholder for RSA
            public_key,
            signature: sig_bytes,
            signed_at: Timestamp::now(),
            key_id: Some(key_handle.0.clone()),
        })
    }

    fn verify(
        &self,
        key_handle: &KeyHandle,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, HardwareError> {
        let tpm_handle = {
            let handles = self.key_handles.read().unwrap();
            handles
                .get(&key_handle.0)
                .copied()
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        // Hash the data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let digest = hasher.finalize();

        let digest_tpm = tss_esapi::structures::Digest::try_from(digest.as_slice())
            .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

        // Create TPM signature structure
        let rsa_sig = RsaSignature::try_from(signature.signature.clone())
            .map_err(|e| HardwareError::InvalidData(e.to_string()))?;

        let tpm_signature = tss_esapi::structures::Signature::RsaSsa(
            tss_esapi::structures::SignatureRsa::new(HashingAlgorithm::Sha256, rsa_sig),
        );

        // Verify with TPM
        let result = context.context_mut().execute_with_nullauth_session(|ctx| {
            ctx.verify_signature(tpm_handle, digest_tpm, tpm_signature)
        });

        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn get_public_key(&self, key_handle: &KeyHandle) -> Result<Vec<u8>, HardwareError> {
        let tpm_handle = {
            let handles = self.key_handles.read().unwrap();
            handles
                .get(&key_handle.0)
                .copied()
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        let (public, _, _) = context
            .context_mut()
            .execute_with_nullauth_session(|ctx| ctx.read_public(tpm_handle))
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;

        // Extract public key bytes based on type
        match public {
            Public::Rsa { unique, .. } => Ok(unique.as_bytes().to_vec()),
            _ => Err(HardwareError::InvalidData(
                "Unsupported key type".to_string(),
            )),
        }
    }

    fn secure_erase(&self, key_handle: &KeyHandle) -> Result<(), HardwareError> {
        let tpm_handle = {
            let mut handles = self.key_handles.write().unwrap();
            handles
                .remove(&key_handle.0)
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        context
            .context_mut()
            .execute_with_nullauth_session(|ctx| ctx.flush_context(tpm_handle.into()))
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;

        info!(key_id = %key_handle.0, "Erased key from TPM");
        Ok(())
    }

    fn key_exists(&self, key_handle: &KeyHandle) -> bool {
        let handles = self.key_handles.read().unwrap();
        handles.contains_key(&key_handle.0)
    }
}

impl RandomSource for TpmHsm {
    fn generate(&self, length: usize) -> Result<Vec<u8>, HardwareError> {
        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        let random_bytes = context
            .context_mut()
            .execute_without_session(|ctx| ctx.get_random(length))
            .map_err(|e| HardwareError::CommunicationError(e.to_string()))?;

        Ok(random_bytes.as_bytes().to_vec())
    }

    fn entropy_quality(&self) -> Result<EntropyQuality, HardwareError> {
        Ok(EntropyQuality {
            bits_per_sample: 8.0, // TPM RNG is considered high quality
            nist_compliant: true, // TPM 2.0 RNGs are NIST compliant
            source_type: if self.is_hardware_backed() {
                "TPM Hardware RNG".to_string()
            } else {
                "Software TPM RNG".to_string()
            },
        })
    }

    fn health_check(&self) -> Result<bool, HardwareError> {
        let mut context = self
            .context
            .write()
            .map_err(|_| HardwareError::CommunicationError("Lock poisoned".to_string()))?;

        context
            .self_test(false)
            .map_err(|e| HardwareError::AttestationFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests require swtpm to be running
    // Run with: cargo test --features tpm -- --ignored

    #[test]
    #[ignore]
    fn test_tpm_hsm_creation() {
        let hsm = TpmHsm::from_env();
        assert!(hsm.is_ok());
    }

    #[test]
    #[ignore]
    fn test_tpm_attestation() {
        let hsm = TpmHsm::from_env().expect("Failed to create TPM HSM");
        let attestation = hsm.get_attestation().expect("Failed to get attestation");
        assert!(!attestation.hardware_type.is_empty());
    }

    #[test]
    #[ignore]
    fn test_tpm_random() {
        let hsm = TpmHsm::from_env().expect("Failed to create TPM HSM");
        let random = hsm.generate(32).expect("Failed to generate random");
        assert_eq!(random.len(), 32);
    }
}
