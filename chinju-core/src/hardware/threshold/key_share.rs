//! Key share management for threshold signatures

use frost_ed25519::keys::{KeyPackage, PublicKeyPackage};
use frost_ed25519::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::FrostError;

/// Serializable key share for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShare {
    /// Participant identifier (1-based index)
    pub participant_id: u16,
    /// Serialized key package bytes
    pub key_package_bytes: Vec<u8>,
    /// Serialized public key package bytes
    pub public_key_package_bytes: Vec<u8>,
    /// Threshold value (t)
    pub threshold: u16,
    /// Total participants (n)
    pub total: u16,
    /// Creation timestamp
    pub created_at: i64,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

impl KeyShare {
    /// Create a new key share from FROST key packages
    pub fn new(
        participant_id: u16,
        key_package: &KeyPackage,
        public_key_package: &PublicKeyPackage,
        threshold: u16,
        total: u16,
    ) -> Result<Self, FrostError> {
        Ok(Self {
            participant_id,
            key_package_bytes: key_package.serialize().map_err(|e| {
                FrostError::FrostLib(format!("Failed to serialize key package: {}", e))
            })?,
            public_key_package_bytes: public_key_package.serialize().map_err(|e| {
                FrostError::FrostLib(format!("Failed to serialize public key package: {}", e))
            })?,
            threshold,
            total,
            created_at: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        })
    }

    /// Deserialize key package
    pub fn key_package(&self) -> Result<KeyPackage, FrostError> {
        KeyPackage::deserialize(&self.key_package_bytes)
            .map_err(|e| FrostError::FrostLib(format!("Failed to deserialize key package: {}", e)))
    }

    /// Deserialize public key package
    pub fn public_key_package(&self) -> Result<PublicKeyPackage, FrostError> {
        PublicKeyPackage::deserialize(&self.public_key_package_bytes).map_err(|e| {
            FrostError::FrostLib(format!("Failed to deserialize public key package: {}", e))
        })
    }

    /// Get the identifier for this participant
    pub fn identifier(&self) -> Result<Identifier, FrostError> {
        Identifier::try_from(self.participant_id)
            .map_err(|e| FrostError::InvalidParticipant(e.to_string()))
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Save to file (JSON format)
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

/// Store for managing multiple key shares
pub struct KeyShareStore {
    shares: HashMap<u16, KeyShare>,
    storage_path: Option<String>,
}

impl KeyShareStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            shares: HashMap::new(),
            storage_path: None,
        }
    }

    /// Create a store with file-based persistence
    pub fn with_storage(path: impl Into<String>) -> Self {
        Self {
            shares: HashMap::new(),
            storage_path: Some(path.into()),
        }
    }

    /// Load shares from storage directory
    pub fn load_from_directory(path: impl AsRef<Path>) -> Result<Self, FrostError> {
        let mut store = Self::with_storage(path.as_ref().to_string_lossy().into_owned());

        if !path.as_ref().exists() {
            return Ok(store);
        }

        for entry in fs::read_dir(path)
            .map_err(|e| FrostError::FrostLib(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry.map_err(|e| FrostError::FrostLib(e.to_string()))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                let share = KeyShare::load_from_file(&path)?;
                store.shares.insert(share.participant_id, share);
            }
        }

        Ok(store)
    }

    /// Add a key share
    pub fn add(&mut self, share: KeyShare) -> Result<(), FrostError> {
        let participant_id = share.participant_id;

        // Save to file if storage is configured
        if let Some(ref storage_path) = self.storage_path {
            let path = Path::new(storage_path);
            if !path.exists() {
                fs::create_dir_all(path)
                    .map_err(|e| FrostError::FrostLib(format!("Failed to create directory: {}", e)))?;
            }
            let file_path = path.join(format!("share_{}.json", participant_id));
            share.save_to_file(file_path)?;
        }

        self.shares.insert(participant_id, share);
        Ok(())
    }

    /// Get a key share by participant ID
    pub fn get(&self, participant_id: u16) -> Option<&KeyShare> {
        self.shares.get(&participant_id)
    }

    /// Get all key shares
    pub fn all(&self) -> Vec<&KeyShare> {
        self.shares.values().collect()
    }

    /// Get number of shares
    pub fn len(&self) -> usize {
        self.shares.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.shares.is_empty()
    }

    /// Remove a key share
    pub fn remove(&mut self, participant_id: u16) -> Option<KeyShare> {
        let share = self.shares.remove(&participant_id);

        // Remove file if storage is configured
        if let (Some(ref storage_path), Some(_)) = (&self.storage_path, &share) {
            let file_path = Path::new(storage_path).join(format!("share_{}.json", participant_id));
            let _ = fs::remove_file(file_path); // Ignore errors
        }

        share
    }

    /// Clear all shares
    pub fn clear(&mut self) {
        // Remove files if storage is configured
        if let Some(ref storage_path) = self.storage_path {
            for participant_id in self.shares.keys() {
                let file_path =
                    Path::new(storage_path).join(format!("share_{}.json", participant_id));
                let _ = fs::remove_file(file_path);
            }
        }

        self.shares.clear();
    }

    /// Get threshold and total from any share
    pub fn get_params(&self) -> Option<(u16, u16)> {
        self.shares.values().next().map(|s| (s.threshold, s.total))
    }
}

impl Default for KeyShareStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_key_share_store() {
        let mut store = KeyShareStore::new();
        assert!(store.is_empty());

        // We can't create real KeyShares without FROST keygen,
        // but we can test the store logic
    }

    #[test]
    fn test_key_share_store_with_storage() {
        let dir = tempdir().unwrap();
        let store = KeyShareStore::with_storage(dir.path().to_string_lossy().into_owned());
        assert!(store.is_empty());
    }
}
