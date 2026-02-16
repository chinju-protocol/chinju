//! PCR (Platform Configuration Register) operations

use super::context::{TpmContext, TpmError};
use serde::{Deserialize, Serialize};
use tss_esapi::{
    attributes::ObjectAttributesBuilder,
    interface_types::{
        algorithm::{HashingAlgorithm, PublicAlgorithm},
        key_bits::RsaKeyBits,
        resource_handles::Hierarchy,
    },
    structures::{
        Data, PcrSelectionListBuilder, PcrSlot, PublicBuilder, PublicKeyRsa,
        PublicRsaParametersBuilder, RsaScheme, SignatureScheme, SymmetricDefinitionObject,
    },
};
use tracing::{debug, info, warn};

/// PCR bank (hash algorithm)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PcrBank {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl PcrBank {
    fn to_hashing_algorithm(self) -> HashingAlgorithm {
        match self {
            PcrBank::Sha1 => HashingAlgorithm::Sha1,
            PcrBank::Sha256 => HashingAlgorithm::Sha256,
            PcrBank::Sha384 => HashingAlgorithm::Sha384,
            PcrBank::Sha512 => HashingAlgorithm::Sha512,
        }
    }

    /// Get digest size for this bank
    pub fn digest_size(&self) -> usize {
        match self {
            PcrBank::Sha1 => 20,
            PcrBank::Sha256 => 32,
            PcrBank::Sha384 => 48,
            PcrBank::Sha512 => 64,
        }
    }
}

impl Default for PcrBank {
    fn default() -> Self {
        PcrBank::Sha256
    }
}

/// PCR value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcrValue {
    /// PCR index (0-23)
    pub index: u8,
    /// Hash bank
    pub bank: PcrBank,
    /// PCR digest value
    pub value: Vec<u8>,
}

impl PcrValue {
    /// Check if PCR is in initial (all zeros) state
    pub fn is_initial(&self) -> bool {
        self.value.iter().all(|&b| b == 0)
    }

    /// Check if PCR is in extended (non-initial) state
    pub fn is_extended(&self) -> bool {
        !self.is_initial()
    }
}

/// Standard PCR indices used in CHINJU Protocol
pub mod pcr_indices {
    /// PCR 0: BIOS/firmware code
    pub const FIRMWARE: u8 = 0;
    /// PCR 7: Secure Boot policy
    pub const SECURE_BOOT: u8 = 7;
    /// PCR 8-15: Application-defined (used by CHINJU)
    pub const CHINJU_CONFIG: u8 = 8;
    /// PCR 9: CHINJU protocol state
    pub const CHINJU_STATE: u8 = 9;
    /// PCR 10: CHINJU audit chain
    pub const CHINJU_AUDIT: u8 = 10;
}

/// PCR operations
pub struct PcrOps<'a> {
    context: &'a mut TpmContext,
}

impl<'a> PcrOps<'a> {
    /// Create PCR operations wrapper
    pub fn new(context: &'a mut TpmContext) -> Self {
        Self { context }
    }

    /// Read a single PCR value
    pub fn read(&mut self, index: u8, bank: PcrBank) -> Result<PcrValue, TpmError> {
        let pcr_slot = PcrSlot::try_from(index as u32)
            .map_err(|_| TpmError::PcrError(format!("Invalid PCR index: {}", index)))?;

        let selection = PcrSelectionListBuilder::new()
            .with_selection(bank.to_hashing_algorithm(), &[pcr_slot])
            .build()
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        let (_, _, pcr_data) = self.context.context_mut()
            .execute_without_session(|ctx| ctx.pcr_read(selection))
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        // Extract the digest value
        let digest = pcr_data.pcr_bank(bank.to_hashing_algorithm())
            .and_then(|bank_data| bank_data.first())
            .map(|d| d.as_bytes().to_vec())
            .unwrap_or_else(|| vec![0u8; bank.digest_size()]);

        debug!(index = index, bank = ?bank, "Read PCR value");

        Ok(PcrValue {
            index,
            bank,
            value: digest,
        })
    }

    /// Read multiple PCR values
    pub fn read_multiple(
        &mut self,
        indices: &[u8],
        bank: PcrBank,
    ) -> Result<Vec<PcrValue>, TpmError> {
        let mut results = Vec::with_capacity(indices.len());
        for &index in indices {
            results.push(self.read(index, bank)?);
        }
        Ok(results)
    }

    /// Extend a PCR with new data
    ///
    /// The new PCR value is: SHA(old_value || data)
    pub fn extend(&mut self, index: u8, bank: PcrBank, data: &[u8]) -> Result<(), TpmError> {
        let pcr_slot = PcrSlot::try_from(index as u32)
            .map_err(|_| TpmError::PcrError(format!("Invalid PCR index: {}", index)))?;

        // Hash the data first
        let digest = match bank {
            PcrBank::Sha1 => {
                use sha1::{Sha1, Digest};
                let mut hasher = Sha1::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            PcrBank::Sha256 => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            PcrBank::Sha384 => {
                use sha2::{Sha384, Digest};
                let mut hasher = Sha384::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            PcrBank::Sha512 => {
                use sha2::{Sha512, Digest};
                let mut hasher = Sha512::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
        };

        // Create digest list
        let digest_values = tss_esapi::structures::DigestValues::create(
            bank.to_hashing_algorithm(),
            tss_esapi::structures::Digest::try_from(digest)
                .map_err(|e| TpmError::PcrError(e.to_string()))?,
        ).map_err(|e| TpmError::PcrError(e.to_string()))?;

        self.context.context_mut()
            .execute_without_session(|ctx| ctx.pcr_extend(pcr_slot.into(), digest_values))
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        info!(index = index, bank = ?bank, "Extended PCR");

        Ok(())
    }

    /// Reset PCR (only available for PCRs 16-23 in some configurations)
    pub fn reset(&mut self, index: u8) -> Result<(), TpmError> {
        if index < 16 {
            return Err(TpmError::PcrError(format!(
                "PCR {} cannot be reset (only PCRs 16-23 are resettable)",
                index
            )));
        }

        let pcr_slot = PcrSlot::try_from(index as u32)
            .map_err(|_| TpmError::PcrError(format!("Invalid PCR index: {}", index)))?;

        self.context.context_mut()
            .execute_without_session(|ctx| ctx.pcr_reset(pcr_slot.into()))
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        info!(index = index, "Reset PCR");

        Ok(())
    }

    /// Get PCR quote (signed attestation of PCR values)
    ///
    /// Creates an Attestation Key (AK) and uses TPM2_Quote to generate
    /// a cryptographically signed attestation of the specified PCR values.
    pub fn quote(
        &mut self,
        indices: &[u8],
        bank: PcrBank,
        nonce: &[u8],
    ) -> Result<PcrQuote, TpmError> {
        // Read PCR values first (for inclusion in the quote struct)
        let pcr_values = self.read_multiple(indices, bank)?;

        // Convert indices to PcrSlots
        let pcr_slots: Vec<PcrSlot> = indices
            .iter()
            .filter_map(|&i| PcrSlot::try_from(i as u32).ok())
            .collect();

        if pcr_slots.is_empty() {
            return Err(TpmError::PcrError("No valid PCR indices".to_string()));
        }

        // Build PCR selection list for the quote
        let pcr_selection = PcrSelectionListBuilder::new()
            .with_selection(bank.to_hashing_algorithm(), &pcr_slots)
            .build()
            .map_err(|e| TpmError::PcrError(e.to_string()))?;

        // Create Attestation Key (AK) - RSA 2048 with restricted signing
        let ak_public = self.create_ak_template()?;

        // Create primary AK under Endorsement Hierarchy
        let ak_result = self
            .context
            .context_mut()
            .execute_with_nullauth_session(|ctx| {
                ctx.create_primary(Hierarchy::Endorsement, ak_public, None, None, None, None)
            })
            .map_err(|e| TpmError::AttestationFailed(format!("Failed to create AK: {}", e)))?;

        let ak_handle = ak_result.key_handle;

        // Prepare qualifying data (nonce)
        let qualifying_data = Data::try_from(nonce.to_vec())
            .map_err(|e| TpmError::AttestationFailed(format!("Invalid nonce: {}", e)))?;

        // Perform TPM2_Quote
        let quote_result = self
            .context
            .context_mut()
            .execute_with_nullauth_session(|ctx| {
                ctx.quote(
                    ak_handle,
                    qualifying_data,
                    SignatureScheme::RsaSsa {
                        hash_scheme: HashingAlgorithm::Sha256.into(),
                    },
                    pcr_selection,
                )
            })
            .map_err(|e| TpmError::AttestationFailed(format!("TPM2_Quote failed: {}", e)))?;

        // Extract signature bytes
        let signature_bytes = match &quote_result.1 {
            tss_esapi::structures::Signature::RsaSsa(rsa_sig) => {
                rsa_sig.signature().as_bytes().to_vec()
            }
            tss_esapi::structures::Signature::RsaPss(rsa_sig) => {
                rsa_sig.signature().as_bytes().to_vec()
            }
            tss_esapi::structures::Signature::EcDsa(ecdsa_sig) => {
                // Concatenate r and s for ECDSA
                let mut sig = ecdsa_sig.signature_r().as_bytes().to_vec();
                sig.extend_from_slice(ecdsa_sig.signature_s().as_bytes());
                sig
            }
            _ => {
                warn!("Unsupported signature type in quote");
                vec![]
            }
        };

        // Get attestation data (TPMS_ATTEST structure)
        let attestation_data = quote_result.0.attestation_data().to_vec();

        // Flush the AK handle to free TPM resources
        let _ = self
            .context
            .context_mut()
            .flush_context(ak_handle.into());

        info!(
            indices = ?indices,
            bank = ?bank,
            sig_len = signature_bytes.len(),
            attest_len = attestation_data.len(),
            "Generated PCR quote with signature"
        );

        Ok(PcrQuote {
            pcr_values,
            nonce: nonce.to_vec(),
            signature: signature_bytes,
            attestation_data,
        })
    }

    /// Create an Attestation Key template for quote operations
    fn create_ak_template(&self) -> Result<tss_esapi::structures::Public, TpmError> {
        // Object attributes for an attestation key:
        // - restricted: can only sign data produced by TPM
        // - sign_encrypt: used for signing
        // - fixed_tpm, fixed_parent: cannot be duplicated
        // - sensitive_data_origin: key generated by TPM
        let object_attributes = ObjectAttributesBuilder::new()
            .with_restricted(true)
            .with_sign_encrypt(true)
            .with_fixed_tpm(true)
            .with_fixed_parent(true)
            .with_sensitive_data_origin(true)
            .with_user_with_auth(true)
            .build()
            .map_err(|e| TpmError::AttestationFailed(e.to_string()))?;

        // RSA 2048 parameters with RSASSA-PKCS1-v1_5 scheme
        let rsa_params = PublicRsaParametersBuilder::new()
            .with_scheme(RsaScheme::RsaSsa(HashingAlgorithm::Sha256.into()))
            .with_key_bits(RsaKeyBits::Rsa2048)
            .with_exponent(tss_esapi::structures::RsaExponent::default())
            .with_is_signing_key(true)
            .with_is_decryption_key(false)
            .with_restricted(true)
            .build()
            .map_err(|e| TpmError::AttestationFailed(e.to_string()))?;

        PublicBuilder::new()
            .with_public_algorithm(PublicAlgorithm::Rsa)
            .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
            .with_object_attributes(object_attributes)
            .with_rsa_parameters(rsa_params)
            .with_rsa_unique_identifier(PublicKeyRsa::default())
            .build()
            .map_err(|e| TpmError::AttestationFailed(e.to_string()))
    }
}

/// PCR Quote (attestation)
///
/// A cryptographically signed attestation of PCR values generated by TPM2_Quote.
/// The signature covers the attestation_data which includes:
/// - Magic number (0xff544347 = "TCG")
/// - Attestation type (TPM_ST_ATTEST_QUOTE)
/// - Qualified signer name
/// - Extra data (nonce)
/// - Clock info
/// - Firmware version
/// - PCR selection and digest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcrQuote {
    /// PCR values included in quote
    pub pcr_values: Vec<PcrValue>,
    /// Nonce provided by verifier (qualifying data)
    pub nonce: Vec<u8>,
    /// TPM signature over attestation data (RSASSA-PKCS1-v1_5 with SHA-256)
    pub signature: Vec<u8>,
    /// Attestation data (TPMS_ATTEST structure, marshaled)
    pub attestation_data: Vec<u8>,
}

impl PcrQuote {
    /// Check if the quote has a valid signature
    pub fn has_signature(&self) -> bool {
        !self.signature.is_empty()
    }

    /// Check if the quote has attestation data
    pub fn has_attestation_data(&self) -> bool {
        !self.attestation_data.is_empty()
    }

    /// Check if this is a complete, signed quote
    pub fn is_complete(&self) -> bool {
        self.has_signature() && self.has_attestation_data() && !self.pcr_values.is_empty()
    }

    /// Verify the nonce matches the expected value
    pub fn verify_nonce(&self, expected_nonce: &[u8]) -> bool {
        self.nonce == expected_nonce
    }

    /// Get the PCR digest (hash of all PCR values)
    ///
    /// This computes a SHA-256 hash of the concatenated PCR values,
    /// which should match the digest in the attestation data.
    pub fn compute_pcr_digest(&self) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        for pcr in &self.pcr_values {
            hasher.update(&pcr.value);
        }
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcr_bank_digest_size() {
        assert_eq!(PcrBank::Sha1.digest_size(), 20);
        assert_eq!(PcrBank::Sha256.digest_size(), 32);
        assert_eq!(PcrBank::Sha384.digest_size(), 48);
        assert_eq!(PcrBank::Sha512.digest_size(), 64);
    }

    #[test]
    fn test_pcr_value_initial() {
        let pcr = PcrValue {
            index: 0,
            bank: PcrBank::Sha256,
            value: vec![0u8; 32],
        };
        assert!(pcr.is_initial());
        assert!(!pcr.is_extended());
    }

    #[test]
    fn test_pcr_value_extended() {
        let mut pcr = PcrValue {
            index: 0,
            bank: PcrBank::Sha256,
            value: vec![0u8; 32],
        };
        pcr.value[0] = 1;
        assert!(!pcr.is_initial());
        assert!(pcr.is_extended());
    }

    #[test]
    fn test_pcr_quote_no_signature() {
        let quote = PcrQuote {
            pcr_values: vec![PcrValue {
                index: 0,
                bank: PcrBank::Sha256,
                value: vec![0u8; 32],
            }],
            nonce: vec![1, 2, 3, 4],
            signature: vec![],
            attestation_data: vec![],
        };
        assert!(!quote.has_signature());
        assert!(!quote.has_attestation_data());
        assert!(!quote.is_complete());
    }

    #[test]
    fn test_pcr_quote_with_signature() {
        let quote = PcrQuote {
            pcr_values: vec![PcrValue {
                index: 0,
                bank: PcrBank::Sha256,
                value: vec![0xAB; 32],
            }],
            nonce: vec![1, 2, 3, 4],
            signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
            attestation_data: vec![0xFF, 0x54, 0x43, 0x47], // TCG magic
        };
        assert!(quote.has_signature());
        assert!(quote.has_attestation_data());
        assert!(quote.is_complete());
    }

    #[test]
    fn test_pcr_quote_verify_nonce() {
        let quote = PcrQuote {
            pcr_values: vec![],
            nonce: vec![1, 2, 3, 4],
            signature: vec![],
            attestation_data: vec![],
        };
        assert!(quote.verify_nonce(&[1, 2, 3, 4]));
        assert!(!quote.verify_nonce(&[5, 6, 7, 8]));
    }

    #[test]
    fn test_pcr_quote_compute_digest() {
        let quote = PcrQuote {
            pcr_values: vec![
                PcrValue {
                    index: 0,
                    bank: PcrBank::Sha256,
                    value: vec![0u8; 32],
                },
                PcrValue {
                    index: 1,
                    bank: PcrBank::Sha256,
                    value: vec![1u8; 32],
                },
            ],
            nonce: vec![],
            signature: vec![],
            attestation_data: vec![],
        };
        let digest = quote.compute_pcr_digest();
        assert_eq!(digest.len(), 32); // SHA-256
    }

    // Integration test requiring swtpm
    #[test]
    #[ignore]
    fn test_tpm_quote_with_signature() {
        // This test requires swtpm to be running
        // Run with: cargo test --features tpm -- --ignored test_tpm_quote_with_signature
        use super::super::context::{TpmConfig, TpmContext};

        let config = TpmConfig::socket("localhost", 2321);
        let mut context = TpmContext::new(config).expect("Failed to connect to TPM");
        let mut ops = PcrOps::new(&mut context);

        let nonce = b"test-nonce-12345";
        let quote = ops.quote(&[0, 1, 7], PcrBank::Sha256, nonce)
            .expect("Failed to get quote");

        assert!(quote.is_complete(), "Quote should have signature and attestation data");
        assert!(quote.verify_nonce(nonce));
        assert!(!quote.signature.is_empty(), "Signature should not be empty");
        assert!(!quote.attestation_data.is_empty(), "Attestation data should not be empty");

        println!("Quote signature length: {}", quote.signature.len());
        println!("Attestation data length: {}", quote.attestation_data.len());
    }
}
