//! SoftHSM HardwareSecurityModule implementation

use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, KeyType, ObjectClass, ObjectHandle};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::hardware::softhsm::session::Pkcs11Session;
use crate::hardware::traits::{
    HardwareError, HardwareSecurityModule, KeyAlgorithm, KeyHandle, TrustRoot,
};
use crate::types::{HardwareAttestation, Signature, SignatureAlgorithm, Timestamp, TrustLevel};

/// SoftHSM implementation of HardwareSecurityModule
pub struct SoftHsm {
    session: Pkcs11Session,
    /// Cache of key handles to PKCS#11 object handles
    key_cache: Arc<RwLock<HashMap<String, (ObjectHandle, ObjectHandle)>>>,
}

impl SoftHsm {
    /// Create a new SoftHSM instance
    pub fn new(module_path: &str, slot: u64, pin: &str) -> Result<Self, HardwareError> {
        let session = Pkcs11Session::new(module_path, slot, pin)?;
        Ok(Self {
            session,
            key_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, HardwareError> {
        let module_path = std::env::var("PKCS11_MODULE").map_err(|_| {
            HardwareError::InvalidData("PKCS11_MODULE environment variable not set".into())
        })?;
        let slot: u64 = std::env::var("PKCS11_SLOT")
            .unwrap_or_else(|_| "0".into())
            .parse()
            .map_err(|_| HardwareError::InvalidData("Invalid PKCS11_SLOT".into()))?;
        let pin = std::env::var("PKCS11_PIN")
            .map_err(|_| HardwareError::InvalidData("PKCS11_PIN not set".into()))?;

        Self::new(&module_path, slot, &pin)
    }

    /// Find a key by label
    fn find_key(&self, label: &str, class: ObjectClass) -> Result<ObjectHandle, HardwareError> {
        let template = vec![
            Attribute::Class(class),
            Attribute::Label(label.as_bytes().to_vec()),
        ];

        let objects = self
            .session
            .session()
            .find_objects(&template)
            .map_err(|e| HardwareError::CommunicationError(format!("Failed to find key: {}", e)))?;

        objects
            .into_iter()
            .next()
            .ok_or_else(|| HardwareError::KeyNotFound(label.to_string()))
    }

    /// Get mechanism for algorithm
    fn get_sign_mechanism(algorithm: KeyAlgorithm) -> Result<Mechanism, HardwareError> {
        match algorithm {
            KeyAlgorithm::Ed25519 => Ok(Mechanism::Eddsa),
            KeyAlgorithm::EcdsaP256 => Ok(Mechanism::Ecdsa),
            KeyAlgorithm::EcdsaP384 => Ok(Mechanism::Ecdsa),
            KeyAlgorithm::Rsa4096 => Ok(Mechanism::RsaPkcs),
        }
    }

    /// Get EC params for algorithm
    fn get_ec_params(algorithm: KeyAlgorithm) -> Result<Vec<u8>, HardwareError> {
        match algorithm {
            // OID for Ed25519: 1.3.101.112
            KeyAlgorithm::Ed25519 => Ok(vec![0x06, 0x03, 0x2b, 0x65, 0x70]),
            // OID for secp256r1 (P-256): 1.2.840.10045.3.1.7
            KeyAlgorithm::EcdsaP256 => {
                Ok(vec![0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07])
            }
            // OID for secp384r1 (P-384): 1.3.132.0.34
            KeyAlgorithm::EcdsaP384 => Ok(vec![0x06, 0x05, 0x2b, 0x81, 0x04, 0x00, 0x22]),
            _ => Err(HardwareError::NotSupported),
        }
    }
}

impl TrustRoot for SoftHsm {
    fn is_hardware_backed(&self) -> bool {
        // SoftHSM is software-based, but still provides HSM interface
        false
    }

    fn security_level(&self) -> TrustLevel {
        TrustLevel::Basic
    }

    fn get_attestation(&self) -> Result<HardwareAttestation, HardwareError> {
        let token_info = self.session.token_info()?;

        Ok(HardwareAttestation {
            device_id: format!("softhsm-{}", self.session.slot().id()),
            firmware_version: "SoftHSM2".to_string(),
            security_level: TrustLevel::Basic,
            attestation_data: token_info.into_bytes(),
            timestamp: Timestamp::now(),
        })
    }
}

impl HardwareSecurityModule for SoftHsm {
    fn generate_key_pair(
        &self,
        algorithm: KeyAlgorithm,
        label: &str,
    ) -> Result<KeyHandle, HardwareError> {
        let key_id = format!("softhsm-{}-{}", label, uuid::Uuid::new_v4());

        match algorithm {
            KeyAlgorithm::Ed25519 | KeyAlgorithm::EcdsaP256 | KeyAlgorithm::EcdsaP384 => {
                let ec_params = Self::get_ec_params(algorithm)?;

                let pub_template = vec![
                    Attribute::Token(true),
                    Attribute::Private(false),
                    Attribute::Label(label.as_bytes().to_vec()),
                    Attribute::Id(key_id.as_bytes().to_vec()),
                    Attribute::Verify(true),
                    Attribute::EcParams(ec_params.clone()),
                ];

                let priv_template = vec![
                    Attribute::Token(true),
                    Attribute::Private(true),
                    Attribute::Label(label.as_bytes().to_vec()),
                    Attribute::Id(key_id.as_bytes().to_vec()),
                    Attribute::Sign(true),
                    Attribute::Sensitive(true),
                    Attribute::Extractable(false),
                ];

                let mechanism = if algorithm == KeyAlgorithm::Ed25519 {
                    Mechanism::EccKeyPairGen
                } else {
                    Mechanism::EccKeyPairGen
                };

                let (pub_handle, priv_handle) = self
                    .session
                    .session()
                    .generate_key_pair(&mechanism, &pub_template, &priv_template)
                    .map_err(|e| {
                        HardwareError::CommunicationError(format!(
                            "Failed to generate key pair: {}",
                            e
                        ))
                    })?;

                // Cache the handles
                {
                    let mut cache = self.key_cache.write().unwrap();
                    cache.insert(key_id.clone(), (pub_handle, priv_handle));
                }

                Ok(KeyHandle::new(key_id))
            }
            KeyAlgorithm::Rsa4096 => {
                let pub_template = vec![
                    Attribute::Token(true),
                    Attribute::Private(false),
                    Attribute::Label(label.as_bytes().to_vec()),
                    Attribute::Id(key_id.as_bytes().to_vec()),
                    Attribute::Verify(true),
                    Attribute::ModulusBits(4096.into()),
                    Attribute::PublicExponent(vec![0x01, 0x00, 0x01]),
                ];

                let priv_template = vec![
                    Attribute::Token(true),
                    Attribute::Private(true),
                    Attribute::Label(label.as_bytes().to_vec()),
                    Attribute::Id(key_id.as_bytes().to_vec()),
                    Attribute::Sign(true),
                    Attribute::Sensitive(true),
                    Attribute::Extractable(false),
                ];

                let (pub_handle, priv_handle) = self
                    .session
                    .session()
                    .generate_key_pair(
                        &Mechanism::RsaPkcsKeyPairGen,
                        &pub_template,
                        &priv_template,
                    )
                    .map_err(|e| {
                        HardwareError::CommunicationError(format!(
                            "Failed to generate RSA key pair: {}",
                            e
                        ))
                    })?;

                // Cache the handles
                {
                    let mut cache = self.key_cache.write().unwrap();
                    cache.insert(key_id.clone(), (pub_handle, priv_handle));
                }

                Ok(KeyHandle::new(key_id))
            }
        }
    }

    fn sign(&self, key_handle: &KeyHandle, data: &[u8]) -> Result<Signature, HardwareError> {
        let priv_handle = {
            let cache = self.key_cache.read().unwrap();
            cache
                .get(&key_handle.0)
                .map(|(_, priv)| *priv)
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        // Determine algorithm from key attributes
        let attrs = self
            .session
            .session()
            .get_attributes(priv_handle, &[AttributeType::KeyType])
            .map_err(|e| {
                HardwareError::CommunicationError(format!("Failed to get key attributes: {}", e))
            })?;

        let key_type = attrs
            .iter()
            .find_map(|a| {
                if let Attribute::KeyType(kt) = a {
                    Some(*kt)
                } else {
                    None
                }
            })
            .ok_or_else(|| HardwareError::InvalidData("Could not determine key type".into()))?;

        let (mechanism, algorithm) = match key_type {
            KeyType::EC => (Mechanism::Ecdsa, SignatureAlgorithm::EcdsaP256),
            KeyType::EC_EDWARDS => (Mechanism::Eddsa, SignatureAlgorithm::Ed25519),
            KeyType::RSA => {
                // RSA signing not yet supported - SignatureAlgorithm lacks RSA variant
                return Err(HardwareError::NotSupported);
            }
            _ => return Err(HardwareError::NotSupported),
        };

        let signature_bytes = self
            .session
            .session()
            .sign(&mechanism, priv_handle, data)
            .map_err(|e| {
                HardwareError::CommunicationError(format!("Failed to sign: {}", e))
            })?;

        let public_key = self.get_public_key(key_handle)?;

        Ok(Signature {
            algorithm,
            public_key,
            signature: signature_bytes,
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
        let pub_handle = {
            let cache = self.key_cache.read().unwrap();
            cache
                .get(&key_handle.0)
                .map(|(pub_h, _)| *pub_h)
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        let mechanism = match signature.algorithm {
            SignatureAlgorithm::Ed25519 => Mechanism::Eddsa,
            SignatureAlgorithm::EcdsaP256 => Mechanism::Ecdsa,
        };

        self.session
            .session()
            .verify(&mechanism, pub_handle, data, &signature.signature)
            .map(|_| true)
            .or_else(|e| {
                // Check if it's a verification failure vs other error
                if e.to_string().contains("SIGNATURE_INVALID") {
                    Ok(false)
                } else {
                    Err(HardwareError::CommunicationError(format!(
                        "Failed to verify: {}",
                        e
                    )))
                }
            })
    }

    fn get_public_key(&self, key_handle: &KeyHandle) -> Result<Vec<u8>, HardwareError> {
        let pub_handle = {
            let cache = self.key_cache.read().unwrap();
            cache
                .get(&key_handle.0)
                .map(|(pub_h, _)| *pub_h)
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        let attrs = self
            .session
            .session()
            .get_attributes(pub_handle, &[AttributeType::EcPoint])
            .map_err(|e| {
                HardwareError::CommunicationError(format!("Failed to get public key: {}", e))
            })?;

        attrs
            .into_iter()
            .find_map(|a| {
                if let Attribute::EcPoint(point) = a {
                    Some(point)
                } else {
                    None
                }
            })
            .ok_or_else(|| HardwareError::InvalidData("Public key not found".into()))
    }

    fn secure_erase(&self, key_handle: &KeyHandle) -> Result<(), HardwareError> {
        let (pub_handle, priv_handle) = {
            let mut cache = self.key_cache.write().unwrap();
            cache
                .remove(&key_handle.0)
                .ok_or_else(|| HardwareError::KeyNotFound(key_handle.0.clone()))?
        };

        // Destroy both keys
        self.session
            .session()
            .destroy_object(priv_handle)
            .map_err(|e| {
                HardwareError::CommunicationError(format!("Failed to destroy private key: {}", e))
            })?;

        self.session
            .session()
            .destroy_object(pub_handle)
            .map_err(|e| {
                HardwareError::CommunicationError(format!("Failed to destroy public key: {}", e))
            })?;

        tracing::info!("Securely erased key: {}", key_handle.0);
        Ok(())
    }

    fn key_exists(&self, key_handle: &KeyHandle) -> bool {
        let cache = self.key_cache.read().unwrap();
        cache.contains_key(&key_handle.0)
    }
}

#[cfg(test)]
mod tests {
    // Tests require SoftHSM to be installed and configured
    // Run with: cargo test --features softhsm -- --ignored

    #[test]
    #[ignore]
    fn test_softhsm_generate_and_sign() {
        use super::*;

        let hsm = SoftHsm::from_env().expect("Failed to create SoftHSM");

        // Generate Ed25519 key
        let key = hsm
            .generate_key_pair(KeyAlgorithm::Ed25519, "test-key")
            .expect("Failed to generate key");

        // Sign data
        let data = b"Hello, CHINJU!";
        let signature = hsm.sign(&key, data).expect("Failed to sign");

        // Verify
        let valid = hsm.verify(&key, data, &signature).expect("Failed to verify");
        assert!(valid);

        // Clean up
        hsm.secure_erase(&key).expect("Failed to erase key");
    }

    #[test]
    #[ignore]
    fn test_softhsm_attestation() {
        use super::*;

        let hsm = SoftHsm::from_env().expect("Failed to create SoftHSM");
        let attestation = hsm.get_attestation().expect("Failed to get attestation");

        assert!(attestation.device_id.starts_with("softhsm-"));
        assert_eq!(attestation.security_level, TrustLevel::Basic);
    }
}
