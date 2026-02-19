//! Ceremony evidence preservation for Genesis Ceremony
//!
//! This module provides structures for preserving cryptographic evidence
//! of the genesis ceremony, including witness signatures, hardware attestations,
//! and optional timestamps.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::ceremony::CeremonyRecord;
use super::FrostError;
use crate::types::{HardwareAttestation, Hash, HashAlgorithm, Signature, Timestamp};

/// Witness record for ceremony evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessRecord {
    /// Witness name or identifier
    pub name: String,
    /// Witness role (e.g., "auditor", "legal", "technical")
    pub role: String,
    /// Witness signature on the ceremony record hash
    pub signature: Signature,
    /// Optional organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    /// Additional notes from witness
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Video recording hash for evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRecordingHash {
    /// Recording identifier
    pub recording_id: String,
    /// Hash of the video file
    pub hash: Hash,
    /// Duration in seconds
    pub duration_seconds: u64,
    /// Recording start timestamp
    pub recorded_at: Timestamp,
    /// Storage location (URL or path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_location: Option<String>,
}

/// External timestamp proof (RFC 3161 or blockchain anchor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampProof {
    /// Proof type
    pub proof_type: TimestampProofType,
    /// Raw proof data
    #[serde(with = "base64_bytes")]
    pub proof_data: Vec<u8>,
    /// External reference (e.g., blockchain tx hash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_reference: Option<String>,
    /// Timestamp of the proof
    pub proved_at: Timestamp,
}

/// Type of timestamp proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampProofType {
    /// RFC 3161 Time Stamp Authority
    Rfc3161,
    /// Bitcoin blockchain anchor
    BitcoinAnchor,
    /// Ethereum blockchain anchor
    EthereumAnchor,
    /// Custom timestamp service
    Custom,
}

/// Complete ceremony evidence package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeremonyEvidence {
    /// Version of the evidence format
    pub version: String,
    /// The ceremony record being attested
    pub record: CeremonyRecord,
    /// Hash of the ceremony record (for signing)
    pub record_hash: Hash,
    /// Witness signatures on the record hash
    #[serde(default)]
    pub witnesses: Vec<WitnessRecord>,
    /// Hardware attestations from ceremony devices
    #[serde(default)]
    pub hardware_attestations: Vec<HardwareAttestation>,
    /// Video recording hashes (if recorded)
    #[serde(default)]
    pub video_recordings: Vec<VideoRecordingHash>,
    /// External timestamp proofs
    #[serde(default)]
    pub timestamp_proofs: Vec<TimestampProof>,
    /// Evidence creation timestamp
    pub created_at: Timestamp,
    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl CeremonyEvidence {
    /// Create new evidence from a ceremony record
    pub fn new(record: CeremonyRecord) -> Result<Self, FrostError> {
        let record_hash = Self::compute_record_hash(&record)?;

        Ok(Self {
            version: "1.0".to_string(),
            record,
            record_hash,
            witnesses: Vec::new(),
            hardware_attestations: Vec::new(),
            video_recordings: Vec::new(),
            timestamp_proofs: Vec::new(),
            created_at: Timestamp::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Compute hash of the ceremony record
    pub fn compute_record_hash(record: &CeremonyRecord) -> Result<Hash, FrostError> {
        let json = serde_json::to_string(record)
            .map_err(|e| FrostError::FrostLib(format!("Failed to serialize record: {}", e)))?;

        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(json.as_bytes());
        let hash_value = hasher.finalize().to_vec();

        Ok(Hash {
            algorithm: HashAlgorithm::Sha3_256,
            value: hash_value,
        })
    }

    /// Add a witness signature
    pub fn add_witness(&mut self, witness: WitnessRecord) {
        self.witnesses.push(witness);
    }

    /// Add a hardware attestation
    pub fn add_hardware_attestation(&mut self, attestation: HardwareAttestation) {
        self.hardware_attestations.push(attestation);
    }

    /// Add a video recording hash
    pub fn add_video_recording(&mut self, recording: VideoRecordingHash) {
        self.video_recordings.push(recording);
    }

    /// Add an external timestamp proof
    pub fn add_timestamp_proof(&mut self, proof: TimestampProof) {
        self.timestamp_proofs.push(proof);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Verify the record hash matches the stored record
    pub fn verify_record_hash(&self) -> Result<bool, FrostError> {
        let computed = Self::compute_record_hash(&self.record)?;
        Ok(computed.value == self.record_hash.value)
    }

    /// Get the record hash as hex string (for display)
    pub fn record_hash_hex(&self) -> String {
        self.record_hash
            .value
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    /// Save evidence to file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), FrostError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| FrostError::FrostLib(format!("Failed to serialize evidence: {}", e)))?;
        fs::write(path, json)
            .map_err(|e| FrostError::FrostLib(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    /// Load evidence from file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, FrostError> {
        let json = fs::read_to_string(path)
            .map_err(|e| FrostError::FrostLib(format!("Failed to read file: {}", e)))?;
        serde_json::from_str(&json)
            .map_err(|e| FrostError::FrostLib(format!("Failed to deserialize evidence: {}", e)))
    }

    /// Check if the evidence is complete (has required components)
    pub fn is_complete(&self) -> bool {
        // Must have completed ceremony
        self.record.phase == super::ceremony::CeremonyPhase::Completed
            // Must have at least one witness
            && !self.witnesses.is_empty()
            // Must have genesis signature
            && self.record.genesis_signature.is_some()
    }

    /// Get summary for display
    pub fn summary(&self) -> EvidenceSummary {
        EvidenceSummary {
            ceremony_id: self.record.ceremony_id.clone(),
            threshold: self.record.threshold,
            total: self.record.total,
            phase: format!("{}", self.record.phase),
            participant_count: self.record.participants.len(),
            witness_count: self.witnesses.len(),
            has_genesis_signature: self.record.genesis_signature.is_some(),
            has_hardware_attestation: !self.hardware_attestations.is_empty(),
            has_timestamp_proof: !self.timestamp_proofs.is_empty(),
            record_hash: self.record_hash_hex(),
            created_at: self.created_at,
        }
    }
}

/// Summary of evidence for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub ceremony_id: String,
    pub threshold: u16,
    pub total: u16,
    pub phase: String,
    pub participant_count: usize,
    pub witness_count: usize,
    pub has_genesis_signature: bool,
    pub has_hardware_attestation: bool,
    pub has_timestamp_proof: bool,
    pub record_hash: String,
    pub created_at: Timestamp,
}

// Helper module for base64 serialization
mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::threshold::ceremony::CeremonyRecord;

    #[test]
    fn test_evidence_creation() {
        let record = CeremonyRecord::new("test-ceremony", 3, 5);
        let evidence = CeremonyEvidence::new(record).unwrap();

        assert_eq!(evidence.version, "1.0");
        assert!(evidence.witnesses.is_empty());
        assert!(evidence.verify_record_hash().unwrap());
    }

    #[test]
    fn test_evidence_hash_consistency() {
        let record = CeremonyRecord::new("test-ceremony", 3, 5);
        let evidence1 = CeremonyEvidence::new(record.clone()).unwrap();
        let evidence2 = CeremonyEvidence::new(record).unwrap();

        // Same record should produce same hash
        assert_eq!(evidence1.record_hash.value, evidence2.record_hash.value);
    }

    #[test]
    fn test_evidence_summary() {
        let record = CeremonyRecord::new("test-ceremony", 3, 5);
        let evidence = CeremonyEvidence::new(record).unwrap();
        let summary = evidence.summary();

        assert_eq!(summary.ceremony_id, "test-ceremony");
        assert_eq!(summary.threshold, 3);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.witness_count, 0);
    }
}
