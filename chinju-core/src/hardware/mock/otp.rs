//! Mock OTP (One-Time Programmable) memory implementation
//!
//! File-based OTP simulation for development and testing.
//! WARNING: NOT suitable for production use - data can be deleted!

use crate::hardware::traits::*;
use crate::types::{HardwareAttestation, Timestamp, TrustLevel};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::warn;

/// Slot state
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum SlotState {
    Empty,
    Burned(Vec<u8>),
}

/// Mock OTP implementation
///
/// Stores data in memory or optionally persists to files.
/// Unlike real OTP, data CAN be deleted (it's a mock).
pub struct MockOtp {
    slots: Arc<RwLock<HashMap<String, SlotState>>>,
    fuses: Arc<RwLock<HashMap<String, bool>>>,
    slot_size: usize,
    max_slots: usize,
    max_fuses: usize,
    storage_path: Option<PathBuf>,
}

impl MockOtp {
    /// Create a new mock OTP with default settings
    pub fn new() -> Self {
        warn!("{}", super::MOCK_WARNING);
        Self {
            slots: Arc::new(RwLock::new(HashMap::new())),
            fuses: Arc::new(RwLock::new(HashMap::new())),
            slot_size: 256, // 256 bytes per slot
            max_slots: 64,
            max_fuses: 256,
            storage_path: None,
        }
    }

    /// Create with file-based persistence
    pub fn with_persistence(storage_path: PathBuf) -> Self {
        warn!("{}", super::MOCK_WARNING);
        let mut otp = Self::new();
        otp.storage_path = Some(storage_path);
        otp
    }

    /// Create with custom slot size
    pub fn with_slot_size(slot_size: usize) -> Self {
        let mut otp = Self::new();
        otp.slot_size = slot_size;
        otp
    }
}

impl Default for MockOtp {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustRoot for MockOtp {
    fn is_hardware_backed(&self) -> bool {
        false
    }

    fn security_level(&self) -> TrustLevel {
        TrustLevel::Mock
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        Ok(HardwareAttestation {
            trust_level: TrustLevel::Mock,
            hardware_type: "MockOTP".to_string(),
            attestation_data: b"MOCK_OTP_NOT_FOR_PRODUCTION".to_vec(),
            manufacturer_signature: None,
            attested_at: Timestamp::now(),
        })
    }
}

impl ImmutableStorage for MockOtp {
    fn read(&self, slot: &SlotId) -> Result<Vec<u8>, HardwareError> {
        let slots = self.slots.read().unwrap();
        match slots.get(&slot.0) {
            Some(SlotState::Burned(data)) => Ok(data.clone()),
            Some(SlotState::Empty) | None => Err(HardwareError::SlotNotFound(slot.0.clone())),
        }
    }

    fn burn(&self, slot: &SlotId, data: &[u8]) -> Result<(), HardwareError> {
        if data.len() > self.slot_size {
            return Err(HardwareError::InvalidData(format!(
                "Data too large: {} > {}",
                data.len(),
                self.slot_size
            )));
        }

        let mut slots = self.slots.write().unwrap();

        // Check if already burned
        if let Some(SlotState::Burned(_)) = slots.get(&slot.0) {
            return Err(HardwareError::SlotAlreadyBurned(slot.0.clone()));
        }

        // Check slot limit
        let burned_count = slots
            .values()
            .filter(|s| matches!(s, SlotState::Burned(_)))
            .count();
        if burned_count >= self.max_slots {
            return Err(HardwareError::StorageFull);
        }

        // Burn the slot
        slots.insert(slot.0.clone(), SlotState::Burned(data.to_vec()));
        tracing::info!(slot_id = %slot.0, size = %data.len(), "Burned data to mock OTP slot");

        // Optionally persist to file
        if let Some(ref path) = self.storage_path {
            let file_path = path.join(format!("{}.bin", slot.0));
            if let Err(e) = std::fs::write(&file_path, data) {
                tracing::warn!(error = %e, "Failed to persist OTP data to file");
            }
        }

        Ok(())
    }

    fn is_burned(&self, slot: &SlotId) -> Result<bool, HardwareError> {
        let slots = self.slots.read().unwrap();
        Ok(matches!(slots.get(&slot.0), Some(SlotState::Burned(_))))
    }

    fn available_slots(&self) -> Result<usize, HardwareError> {
        let slots = self.slots.read().unwrap();
        let burned_count = slots
            .values()
            .filter(|s| matches!(s, SlotState::Burned(_)))
            .count();
        Ok(self.max_slots - burned_count)
    }

    fn slot_size(&self) -> usize {
        self.slot_size
    }
}

impl FuseControl for MockOtp {
    fn burn_fuse(&self, fuse_id: &FuseId) -> Result<(), HardwareError> {
        let mut fuses = self.fuses.write().unwrap();

        // Check if already blown
        if fuses.get(&fuse_id.0) == Some(&true) {
            return Err(HardwareError::FuseAlreadyBlown(fuse_id.0.clone()));
        }

        // Check fuse limit
        let blown_count = fuses.values().filter(|&&b| b).count();
        if blown_count >= self.max_fuses {
            return Err(HardwareError::StorageFull);
        }

        // Blow the fuse
        fuses.insert(fuse_id.0.clone(), true);
        tracing::info!(fuse_id = %fuse_id.0, "Burned mock eFuse (IRREVERSIBLE in real hardware)");

        Ok(())
    }

    fn is_fuse_blown(&self, fuse_id: &FuseId) -> Result<bool, HardwareError> {
        let fuses = self.fuses.read().unwrap();
        Ok(fuses.get(&fuse_id.0) == Some(&true))
    }

    fn fuse_count(&self) -> usize {
        self.max_fuses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_otp_burn_and_read() {
        let otp = MockOtp::new();
        let slot = SlotId::new("test-slot-1");
        let data = b"genesis-hash-12345";

        // Burn data
        otp.burn(&slot, data).unwrap();

        // Read back
        let read_data = otp.read(&slot).unwrap();
        assert_eq!(read_data, data);

        // Cannot burn again
        let result = otp.burn(&slot, b"different-data");
        assert!(matches!(result, Err(HardwareError::SlotAlreadyBurned(_))));
    }

    #[test]
    fn test_mock_otp_fuse() {
        let otp = MockOtp::new();
        let fuse = FuseId::new("user-revoke-123");

        // Not blown initially
        assert!(!otp.is_fuse_blown(&fuse).unwrap());

        // Blow fuse
        otp.burn_fuse(&fuse).unwrap();

        // Now blown
        assert!(otp.is_fuse_blown(&fuse).unwrap());

        // Cannot blow again
        let result = otp.burn_fuse(&fuse);
        assert!(matches!(result, Err(HardwareError::FuseAlreadyBlown(_))));
    }

    #[test]
    fn test_mock_otp_slot_size_limit() {
        let otp = MockOtp::with_slot_size(16);
        let slot = SlotId::new("small-slot");

        // Data too large
        let large_data = vec![0u8; 32];
        let result = otp.burn(&slot, &large_data);
        assert!(matches!(result, Err(HardwareError::InvalidData(_))));

        // Data within limit
        let small_data = vec![0u8; 16];
        otp.burn(&slot, &small_data).unwrap();
    }
}
