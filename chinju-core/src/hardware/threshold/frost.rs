//! FROST threshold signature implementation

use frost_ed25519 as frost;
use frost_ed25519::keys::{KeyPackage, PublicKeyPackage, SecretShare};
use frost_ed25519::round1::{SigningCommitments, SigningNonces};
use frost_ed25519::round2::SignatureShare;
use frost_ed25519::{Identifier, Signature, SigningPackage};
use rand::rngs::OsRng;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// FROST-related errors
#[derive(Debug, Error)]
pub enum FrostError {
    #[error("Invalid threshold: threshold must be <= total and > 0")]
    InvalidThreshold,

    #[error("Not enough participants: need {needed}, got {got}")]
    NotEnoughParticipants { needed: u16, got: usize },

    #[error("DKG not completed")]
    DkgNotCompleted,

    #[error("Invalid participant: {0}")]
    InvalidParticipant(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("FROST error: {0}")]
    FrostLib(String),
}

impl From<frost::Error> for FrostError {
    fn from(e: frost::Error) -> Self {
        FrostError::FrostLib(e.to_string())
    }
}

/// FROST coordinator for managing threshold signatures
pub struct FrostCoordinator {
    threshold: u16,
    total: u16,
    /// Key packages for each participant (after DKG)
    key_packages: Arc<RwLock<HashMap<Identifier, KeyPackage>>>,
    /// Public key package (shared among all participants)
    pub public_key_package: Arc<RwLock<Option<PublicKeyPackage>>>,
}

impl FrostCoordinator {
    /// Create a new FROST coordinator
    ///
    /// # Arguments
    /// * `threshold` - Minimum number of signers required (t)
    /// * `total` - Total number of participants (n)
    pub fn new(threshold: u16, total: u16) -> Result<Self, FrostError> {
        if threshold == 0 || threshold > total {
            return Err(FrostError::InvalidThreshold);
        }

        Ok(Self {
            threshold,
            total,
            key_packages: Arc::new(RwLock::new(HashMap::new())),
            public_key_package: Arc::new(RwLock::new(None)),
        })
    }

    /// Get threshold value
    pub fn threshold(&self) -> u16 {
        self.threshold
    }

    /// Get total participants
    pub fn total(&self) -> u16 {
        self.total
    }

    /// Perform trusted dealer key generation
    ///
    /// This is a simplified DKG that uses a trusted dealer to generate shares.
    /// For production use, implement proper distributed key generation.
    pub fn trusted_dealer_keygen(&self) -> Result<Vec<(Identifier, SecretShare)>, FrostError> {
        let (shares, pubkey_package) = frost::keys::generate_with_dealer(
            self.total,
            self.threshold,
            frost::keys::IdentifierList::Default,
            &mut OsRng,
        )?;

        // Store public key package
        {
            let mut pkg = self.public_key_package.write().unwrap();
            *pkg = Some(pubkey_package);
        }

        // Convert shares to key packages and store them
        {
            let mut key_pkgs = self.key_packages.write().unwrap();
            for (id, share) in &shares {
                let key_pkg = KeyPackage::try_from(share.clone())?;
                key_pkgs.insert(*id, key_pkg);
            }
        }

        Ok(shares.into_iter().collect())
    }

    /// Import key packages (for resuming from stored shares)
    pub fn import_key_packages(
        &self,
        packages: Vec<(Identifier, KeyPackage)>,
        pubkey_package: PublicKeyPackage,
    ) -> Result<(), FrostError> {
        {
            let mut key_pkgs = self.key_packages.write().unwrap();
            for (id, pkg) in packages {
                key_pkgs.insert(id, pkg);
            }
        }

        {
            let mut pkg = self.public_key_package.write().unwrap();
            *pkg = Some(pubkey_package);
        }

        Ok(())
    }

    /// Check if DKG has been completed
    pub fn is_dkg_complete(&self) -> bool {
        self.public_key_package.read().unwrap().is_some()
    }

    /// Get the group public key
    pub fn group_public_key(&self) -> Result<Vec<u8>, FrostError> {
        let pkg = self.public_key_package.read().unwrap();
        let pubkey_pkg = pkg.as_ref().ok_or(FrostError::DkgNotCompleted)?;
        Ok(pubkey_pkg.verifying_key().serialize()?.to_vec())
    }

    /// Sign a message with the specified participants
    ///
    /// # Arguments
    /// * `signer_ids` - Identifiers of participants who will sign
    /// * `message` - Message to sign
    pub fn sign(&self, signer_ids: &[Identifier], message: &[u8]) -> Result<Vec<u8>, FrostError> {
        if signer_ids.len() < self.threshold as usize {
            return Err(FrostError::NotEnoughParticipants {
                needed: self.threshold,
                got: signer_ids.len(),
            });
        }

        let key_packages = self.key_packages.read().unwrap();
        let pubkey_package = self
            .public_key_package
            .read()
            .unwrap()
            .clone()
            .ok_or(FrostError::DkgNotCompleted)?;

        // Round 1: Generate commitments and nonces
        let mut nonces_map: BTreeMap<Identifier, SigningNonces> = BTreeMap::new();
        let mut commitments_map: BTreeMap<Identifier, SigningCommitments> = BTreeMap::new();

        for id in signer_ids {
            let key_pkg = key_packages
                .get(id)
                .ok_or_else(|| FrostError::InvalidParticipant(format!("{:?}", id)))?;

            let (nonces, commitments) = frost::round1::commit(key_pkg.signing_share(), &mut OsRng);
            nonces_map.insert(*id, nonces);
            commitments_map.insert(*id, commitments);
        }

        // Create signing package
        let signing_package = SigningPackage::new(commitments_map, message);

        // Round 2: Generate signature shares
        let mut signature_shares: BTreeMap<Identifier, SignatureShare> = BTreeMap::new();

        for id in signer_ids {
            let key_pkg = key_packages.get(id).unwrap();
            let nonces = nonces_map.get(id).unwrap();

            let sig_share = frost::round2::sign(&signing_package, nonces, key_pkg)?;
            signature_shares.insert(*id, sig_share);
        }

        // Aggregate signature shares
        let group_signature = frost::aggregate(&signing_package, &signature_shares, &pubkey_package)?;

        // Verify the signature
        pubkey_package
            .verifying_key()
            .verify(message, &group_signature)
            .map_err(|e| FrostError::VerificationFailed(e.to_string()))?;

        Ok(group_signature.serialize()?.to_vec())
    }

    /// Verify a signature against the group public key
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, FrostError> {
        let pubkey_package = self
            .public_key_package
            .read()
            .unwrap()
            .clone()
            .ok_or(FrostError::DkgNotCompleted)?;

        let sig = Signature::deserialize(signature.try_into().map_err(|_| {
            FrostError::VerificationFailed("Invalid signature length".into())
        })?)?;

        match pubkey_package.verifying_key().verify(message, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify a signature using a raw group public key bytes
    ///
    /// This static method allows verification without a full coordinator instance,
    /// useful for verifier-only nodes that just have the group public key.
    pub fn verify_with_pubkey(pubkey_bytes: &[u8], message: &[u8], signature: &[u8]) -> Result<bool, FrostError> {
        let verifying_key = frost_ed25519::VerifyingKey::deserialize(
            pubkey_bytes.try_into().map_err(|_| FrostError::VerificationFailed("Invalid public key length".into()))?
        ).map_err(|e| FrostError::VerificationFailed(e.to_string()))?;

        let sig = Signature::deserialize(signature.try_into().map_err(|_| {
            FrostError::VerificationFailed("Invalid signature length".into())
        })?)?;

        match verifying_key.verify(message, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get participant identifiers
    pub fn participant_ids(&self) -> Vec<Identifier> {
        self.key_packages.read().unwrap().keys().cloned().collect()
    }
}

/// FROST participant for distributed signing
pub struct FrostParticipant {
    identifier: Identifier,
    key_package: KeyPackage,
    public_key_package: PublicKeyPackage,
}

impl FrostParticipant {
    /// Create a new participant from key shares
    pub fn new(
        identifier: Identifier,
        key_package: KeyPackage,
        public_key_package: PublicKeyPackage,
    ) -> Self {
        Self {
            identifier,
            key_package,
            public_key_package,
        }
    }

    /// Get the participant's identifier
    pub fn identifier(&self) -> Identifier {
        self.identifier
    }

    /// Generate round 1 commitment
    pub fn commit(&self) -> (SigningNonces, SigningCommitments) {
        frost::round1::commit(self.key_package.signing_share(), &mut OsRng)
    }

    /// Generate signature share for round 2
    pub fn sign(
        &self,
        signing_package: &SigningPackage,
        nonces: &SigningNonces,
    ) -> Result<SignatureShare, FrostError> {
        Ok(frost::round2::sign(
            signing_package,
            nonces,
            &self.key_package,
        )?)
    }

    /// Get the group public key
    pub fn group_public_key(&self) -> Result<Vec<u8>, FrostError> {
        Ok(self.public_key_package.verifying_key().serialize()?.to_vec())
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, FrostError> {
        let sig = Signature::deserialize(signature.try_into().map_err(|_| {
            FrostError::VerificationFailed("Invalid signature length".into())
        })?)?;

        match self.public_key_package.verifying_key().verify(message, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frost_coordinator_creation() {
        let coordinator = FrostCoordinator::new(3, 5).unwrap();
        assert_eq!(coordinator.threshold(), 3);
        assert_eq!(coordinator.total(), 5);
        assert!(!coordinator.is_dkg_complete());
    }

    #[test]
    fn test_frost_invalid_threshold() {
        assert!(FrostCoordinator::new(0, 5).is_err());
        assert!(FrostCoordinator::new(6, 5).is_err());
    }

    #[test]
    fn test_frost_trusted_dealer_keygen() {
        let coordinator = FrostCoordinator::new(2, 3).unwrap();
        let shares = coordinator.trusted_dealer_keygen().unwrap();

        assert_eq!(shares.len(), 3);
        assert!(coordinator.is_dkg_complete());

        let pubkey = coordinator.group_public_key().unwrap();
        assert!(!pubkey.is_empty());
    }

    #[test]
    fn test_frost_sign_and_verify() {
        let coordinator = FrostCoordinator::new(2, 3).unwrap();
        let shares = coordinator.trusted_dealer_keygen().unwrap();

        let message = b"Hello, CHINJU Protocol!";

        // Get first 2 signers (meets threshold)
        let signer_ids: Vec<_> = shares.iter().take(2).map(|(id, _)| *id).collect();

        let signature = coordinator.sign(&signer_ids, message).unwrap();
        assert!(!signature.is_empty());

        // Verify the signature
        let valid = coordinator.verify(message, &signature).unwrap();
        assert!(valid);

        // Verify with wrong message fails
        let wrong_message = b"Wrong message";
        let valid = coordinator.verify(wrong_message, &signature).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_frost_not_enough_signers() {
        let coordinator = FrostCoordinator::new(3, 5).unwrap();
        let shares = coordinator.trusted_dealer_keygen().unwrap();

        let message = b"Test message";

        // Only 2 signers (below threshold of 3)
        let signer_ids: Vec<_> = shares.iter().take(2).map(|(id, _)| *id).collect();

        let result = coordinator.sign(&signer_ids, message);
        assert!(matches!(result, Err(FrostError::NotEnoughParticipants { .. })));
    }
}
