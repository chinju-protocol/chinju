//! Genesis ceremony management for threshold key generation

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::key_share::{KeyShare, KeyShareStore};
use super::FrostCoordinator;
use super::FrostError;

/// Ceremony phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CeremonyPhase {
    /// Ceremony not started
    NotStarted,
    /// Participants are registering
    Registration,
    /// Key generation in progress
    KeyGeneration,
    /// Verification and signing of genesis hash
    Verification,
    /// Ceremony completed successfully
    Completed,
    /// Ceremony failed
    Failed,
}

impl std::fmt::Display for CeremonyPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "Not Started"),
            Self::Registration => write!(f, "Registration"),
            Self::KeyGeneration => write!(f, "Key Generation"),
            Self::Verification => write!(f, "Verification"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Participant record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantRecord {
    /// Participant ID (1-based)
    pub id: u16,
    /// Display name
    pub name: String,
    /// Public key (after key generation)
    pub public_key: Option<Vec<u8>>,
    /// Registration timestamp
    pub registered_at: i64,
    /// Whether this participant is ready for next phase
    pub ready: bool,
}

/// Ceremony record for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeremonyRecord {
    /// Unique ceremony ID
    pub ceremony_id: String,
    /// Threshold value (t)
    pub threshold: u16,
    /// Total participants (n)
    pub total: u16,
    /// Current phase
    pub phase: CeremonyPhase,
    /// Registered participants
    pub participants: Vec<ParticipantRecord>,
    /// Group public key (after key generation)
    pub group_public_key: Option<Vec<u8>>,
    /// Genesis hash to sign
    pub genesis_hash: Option<Vec<u8>>,
    /// Genesis signature
    pub genesis_signature: Option<Vec<u8>>,
    /// Start timestamp
    pub started_at: i64,
    /// Completion timestamp
    pub completed_at: Option<i64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional notes
    pub notes: Vec<String>,
}

impl CeremonyRecord {
    /// Create a new ceremony record
    pub fn new(ceremony_id: impl Into<String>, threshold: u16, total: u16) -> Self {
        Self {
            ceremony_id: ceremony_id.into(),
            threshold,
            total,
            phase: CeremonyPhase::NotStarted,
            participants: Vec::new(),
            group_public_key: None,
            genesis_hash: None,
            genesis_signature: None,
            started_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            error: None,
            notes: Vec::new(),
        }
    }

    /// Add a note to the record
    pub fn add_note(&mut self, note: impl Into<String>) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        self.notes.push(format!("[{}] {}", timestamp, note.into()));
    }

    /// Save to file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), FrostError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| FrostError::FrostLib(format!("Failed to serialize: {}", e)))?;
        fs::write(path, json)
            .map_err(|e| FrostError::FrostLib(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    /// Load from file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, FrostError> {
        let json = fs::read_to_string(path)
            .map_err(|e| FrostError::FrostLib(format!("Failed to read file: {}", e)))?;
        serde_json::from_str(&json)
            .map_err(|e| FrostError::FrostLib(format!("Failed to deserialize: {}", e)))
    }
}

/// Genesis ceremony manager
pub struct Ceremony {
    record: CeremonyRecord,
    coordinator: Option<FrostCoordinator>,
    key_store: KeyShareStore,
}

impl Ceremony {
    /// Create a new ceremony
    pub fn new(
        ceremony_id: impl Into<String>,
        threshold: u16,
        total: u16,
    ) -> Result<Self, FrostError> {
        if threshold == 0 || threshold > total {
            return Err(FrostError::InvalidThreshold);
        }

        let record = CeremonyRecord::new(ceremony_id, threshold, total);

        Ok(Self {
            record,
            coordinator: None,
            key_store: KeyShareStore::new(),
        })
    }

    /// Create ceremony with file storage
    pub fn with_storage(
        ceremony_id: impl Into<String>,
        threshold: u16,
        total: u16,
        storage_path: impl Into<String>,
    ) -> Result<Self, FrostError> {
        let mut ceremony = Self::new(ceremony_id, threshold, total)?;
        ceremony.key_store = KeyShareStore::with_storage(storage_path);
        Ok(ceremony)
    }

    /// Restore ceremony from record
    pub fn from_record(record: CeremonyRecord, storage_path: impl Into<String>) -> Self {
        Self {
            record,
            coordinator: None,
            key_store: KeyShareStore::with_storage(storage_path),
        }
    }

    /// Get the ceremony record
    pub fn record(&self) -> &CeremonyRecord {
        &self.record
    }

    /// Get current phase
    pub fn phase(&self) -> CeremonyPhase {
        self.record.phase
    }

    /// Start registration phase
    pub fn start_registration(&mut self) -> Result<(), FrostError> {
        if self.record.phase != CeremonyPhase::NotStarted {
            return Err(FrostError::FrostLib("Ceremony already started".into()));
        }

        self.record.phase = CeremonyPhase::Registration;
        self.record.add_note("Registration phase started");
        Ok(())
    }

    /// Register a participant
    pub fn register_participant(&mut self, name: impl Into<String>) -> Result<u16, FrostError> {
        if self.record.phase != CeremonyPhase::Registration {
            return Err(FrostError::FrostLib("Not in registration phase".into()));
        }

        if self.record.participants.len() >= self.record.total as usize {
            return Err(FrostError::FrostLib("Maximum participants reached".into()));
        }

        let id = self.record.participants.len() as u16 + 1;
        let participant = ParticipantRecord {
            id,
            name: name.into(),
            public_key: None,
            registered_at: chrono::Utc::now().timestamp(),
            ready: false,
        };

        self.record.add_note(format!(
            "Participant {} registered: {}",
            id, participant.name
        ));
        self.record.participants.push(participant);

        Ok(id)
    }

    /// Start key generation phase
    pub fn start_key_generation(&mut self) -> Result<(), FrostError> {
        if self.record.phase != CeremonyPhase::Registration {
            return Err(FrostError::FrostLib("Not in registration phase".into()));
        }

        if self.record.participants.len() < self.record.total as usize {
            return Err(FrostError::NotEnoughParticipants {
                needed: self.record.total,
                got: self.record.participants.len(),
            });
        }

        self.record.phase = CeremonyPhase::KeyGeneration;
        self.record.add_note("Key generation phase started");

        // Create coordinator
        self.coordinator = Some(FrostCoordinator::new(
            self.record.threshold,
            self.record.total,
        )?);

        Ok(())
    }

    /// Run trusted dealer key generation
    ///
    /// This is a simplified key generation that uses a trusted dealer.
    /// For production, implement proper distributed key generation.
    pub fn run_trusted_dealer_keygen(&mut self) -> Result<(), FrostError> {
        if self.record.phase != CeremonyPhase::KeyGeneration {
            return Err(FrostError::FrostLib("Not in key generation phase".into()));
        }

        let coordinator = self
            .coordinator
            .as_ref()
            .ok_or_else(|| FrostError::FrostLib("Coordinator not initialized".into()))?;

        // Generate keys
        let shares = coordinator.trusted_dealer_keygen()?;

        // Get public key package
        let group_pubkey = coordinator.group_public_key()?;
        self.record.group_public_key = Some(group_pubkey);

        // Store key shares
        for (idx, (_id, share)) in shares.iter().enumerate() {
            let key_pkg = frost_ed25519::keys::KeyPackage::try_from(share.clone())?;
            let pubkey_pkg = coordinator
                .public_key_package
                .read()
                .unwrap()
                .clone()
                .unwrap();

            // participant_id is 1-based
            let participant_id = (idx + 1) as u16;

            let key_share = KeyShare::new(
                participant_id,
                &key_pkg,
                &pubkey_pkg,
                self.record.threshold,
                self.record.total,
            )?;

            self.key_store.add(key_share)?;
        }

        self.record.add_note(format!(
            "Key generation completed. {} shares created.",
            shares.len()
        ));

        self.record.phase = CeremonyPhase::Verification;
        Ok(())
    }

    /// Set genesis hash to sign
    pub fn set_genesis_hash(&mut self, hash: Vec<u8>) -> Result<(), FrostError> {
        if self.record.phase != CeremonyPhase::Verification {
            return Err(FrostError::FrostLib("Not in verification phase".into()));
        }

        self.record.genesis_hash = Some(hash);
        self.record.add_note("Genesis hash set for signing");
        Ok(())
    }

    /// Sign genesis hash with threshold signature
    pub fn sign_genesis(&mut self) -> Result<Vec<u8>, FrostError> {
        if self.record.phase != CeremonyPhase::Verification {
            return Err(FrostError::FrostLib("Not in verification phase".into()));
        }

        let genesis_hash = self
            .record
            .genesis_hash
            .as_ref()
            .ok_or_else(|| FrostError::FrostLib("Genesis hash not set".into()))?;

        let coordinator = self
            .coordinator
            .as_ref()
            .ok_or_else(|| FrostError::FrostLib("Coordinator not initialized".into()))?;

        // Get all participant IDs
        let signer_ids = coordinator.participant_ids();

        // Sign with all participants
        let signature = coordinator.sign(&signer_ids, genesis_hash)?;

        self.record.genesis_signature = Some(signature.clone());
        self.record.add_note("Genesis hash signed");

        Ok(signature)
    }

    /// Complete the ceremony
    pub fn complete(&mut self) -> Result<(), FrostError> {
        if self.record.phase != CeremonyPhase::Verification {
            return Err(FrostError::FrostLib("Not in verification phase".into()));
        }

        if self.record.genesis_signature.is_none() {
            return Err(FrostError::FrostLib("Genesis not signed yet".into()));
        }

        self.record.phase = CeremonyPhase::Completed;
        self.record.completed_at = Some(chrono::Utc::now().timestamp());
        self.record.add_note("Ceremony completed successfully");

        Ok(())
    }

    /// Mark ceremony as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        let error_msg = error.into();
        self.record.phase = CeremonyPhase::Failed;
        self.record.completed_at = Some(chrono::Utc::now().timestamp());
        self.record.error = Some(error_msg.clone());
        self.record
            .add_note(format!("Ceremony failed: {}", error_msg));
    }

    /// Get key shares (for distribution to participants)
    pub fn get_key_shares(&self) -> &KeyShareStore {
        &self.key_store
    }

    /// Restore coordinator from stored key shares (Trusted Dealer mode only)
    ///
    /// This restores the FrostCoordinator from the stored key shares,
    /// allowing the ceremony to resume signing operations after restart.
    pub fn restore_coordinator(&mut self) -> Result<(), FrostError> {
        if self.record.phase == CeremonyPhase::NotStarted
            || self.record.phase == CeremonyPhase::Registration
        {
            return Err(FrostError::FrostLib("Key generation not completed".into()));
        }

        // Create new coordinator
        let coordinator = FrostCoordinator::new(self.record.threshold, self.record.total)?;

        // Load all key shares and import into coordinator
        let mut key_packages = Vec::new();
        let mut pubkey_pkg = None;

        for participant in &self.record.participants {
            if let Some(share) = self.key_store.get(participant.id) {
                // Get KeyPackage from KeyShare
                let key_pkg = share.key_package()?;
                let id = share.identifier()?;
                key_packages.push((id, key_pkg));

                // Get public key package (same for all participants)
                if pubkey_pkg.is_none() {
                    pubkey_pkg = Some(share.public_key_package()?);
                }
            }
        }

        if key_packages.is_empty() {
            return Err(FrostError::FrostLib("No key shares found".into()));
        }

        let pubkey_pkg =
            pubkey_pkg.ok_or_else(|| FrostError::FrostLib("No public key package found".into()))?;

        // Import into coordinator
        coordinator.import_key_packages(key_packages, pubkey_pkg)?;

        self.coordinator = Some(coordinator);
        self.record.add_note("Coordinator restored from key shares");

        Ok(())
    }

    /// Create CeremonyEvidence from the current state
    pub fn create_evidence(&self) -> Result<super::evidence::CeremonyEvidence, FrostError> {
        super::evidence::CeremonyEvidence::new(self.record.clone())
    }

    /// Export a specific key share for distribution to participant
    pub fn export_key_share(&self, participant_id: u16) -> Result<Option<&KeyShare>, FrostError> {
        Ok(self.key_store.get(participant_id))
    }

    /// Get all key share IDs
    pub fn key_share_ids(&self) -> Vec<u16> {
        self.key_store
            .all()
            .iter()
            .map(|s| s.participant_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceremony_creation() {
        let ceremony = Ceremony::new("test-ceremony", 3, 5).unwrap();
        assert_eq!(ceremony.phase(), CeremonyPhase::NotStarted);
        assert_eq!(ceremony.record().threshold, 3);
        assert_eq!(ceremony.record().total, 5);
    }

    #[test]
    fn test_ceremony_workflow() {
        let mut ceremony = Ceremony::new("test-ceremony", 2, 3).unwrap();

        // Start registration
        ceremony.start_registration().unwrap();
        assert_eq!(ceremony.phase(), CeremonyPhase::Registration);

        // Register participants
        let id1 = ceremony.register_participant("Alice").unwrap();
        let id2 = ceremony.register_participant("Bob").unwrap();
        let id3 = ceremony.register_participant("Charlie").unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_eq!(ceremony.record().participants.len(), 3);

        // Start key generation
        ceremony.start_key_generation().unwrap();
        assert_eq!(ceremony.phase(), CeremonyPhase::KeyGeneration);

        // Run trusted dealer keygen
        ceremony.run_trusted_dealer_keygen().unwrap();
        assert_eq!(ceremony.phase(), CeremonyPhase::Verification);
        assert!(ceremony.record().group_public_key.is_some());

        // Set and sign genesis hash
        let genesis_hash = b"genesis-hash-example".to_vec();
        ceremony.set_genesis_hash(genesis_hash).unwrap();

        let signature = ceremony.sign_genesis().unwrap();
        assert!(!signature.is_empty());

        // Complete ceremony
        ceremony.complete().unwrap();
        assert_eq!(ceremony.phase(), CeremonyPhase::Completed);
        assert!(ceremony.record().completed_at.is_some());
    }
}
