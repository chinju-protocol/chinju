//! PCR (Platform Configuration Register) operations

use super::context::{TpmContext, TpmError};
use serde::{Deserialize, Serialize};
use tss_esapi::{
    interface_types::algorithm::HashingAlgorithm,
    structures::{PcrSelectionListBuilder, PcrSlot},
};
use tracing::{debug, info};

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
    pub fn quote(
        &mut self,
        indices: &[u8],
        bank: PcrBank,
        nonce: &[u8],
    ) -> Result<PcrQuote, TpmError> {
        // For a full implementation, we would:
        // 1. Create an attestation key (AK)
        // 2. Use TPM2_Quote to get signed PCR values
        // This is a placeholder for the full implementation

        let pcr_values = self.read_multiple(indices, bank)?;

        Ok(PcrQuote {
            pcr_values,
            nonce: nonce.to_vec(),
            signature: vec![], // Would be filled by TPM2_Quote
            attestation_data: vec![], // Would be filled by TPM2_Quote
        })
    }
}

/// PCR Quote (attestation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcrQuote {
    /// PCR values included in quote
    pub pcr_values: Vec<PcrValue>,
    /// Nonce provided by verifier
    pub nonce: Vec<u8>,
    /// TPM signature over attestation data
    pub signature: Vec<u8>,
    /// Attestation data (TPMS_ATTEST structure)
    pub attestation_data: Vec<u8>,
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
}
