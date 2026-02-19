//! Request handlers for Enclave operations

use crate::key_manager::KeyManager;
use crate::nsm::NsmClient;
use crate::protocol::{EnclaveRequest, EnclaveResponse};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Request handler
pub struct RequestHandler {
    key_manager: KeyManager,
    nsm: NsmClient,
}

impl RequestHandler {
    /// Create a new request handler
    pub fn new() -> Self {
        Self {
            key_manager: KeyManager::new(),
            nsm: NsmClient::default(),
        }
    }

    /// Handle an incoming request
    pub fn handle(&self, request: EnclaveRequest) -> EnclaveResponse {
        match request {
            EnclaveRequest::GetAttestation {
                challenge,
                user_data,
            } => self.handle_get_attestation(challenge, user_data),

            EnclaveRequest::Seal { data } => self.handle_seal(data),

            EnclaveRequest::Unseal { sealed_data } => self.handle_unseal(sealed_data),

            EnclaveRequest::Sign { key_id, data } => self.handle_sign(key_id, data),

            EnclaveRequest::GenerateKeyPair { algorithm, label } => {
                self.handle_generate_key_pair(algorithm, label)
            }

            EnclaveRequest::GetPublicKey { key_id } => self.handle_get_public_key(key_id),

            EnclaveRequest::DeleteKey { key_id } => self.handle_delete_key(key_id),

            EnclaveRequest::HealthCheck => self.handle_health_check(),

            EnclaveRequest::Heartbeat => self.handle_heartbeat(),

            EnclaveRequest::GetStatus => self.handle_get_status(),
        }
    }

    fn handle_get_attestation(
        &self,
        challenge: Vec<u8>,
        user_data: Option<Vec<u8>>,
    ) -> EnclaveResponse {
        debug!("Handling GetAttestation request");

        match self
            .nsm
            .get_attestation_document(user_data.as_deref(), Some(&challenge), None)
        {
            Ok(document) => EnclaveResponse::Attestation { document },
            Err(e) => {
                error!("Attestation error: {}", e);
                EnclaveResponse::error(
                    "ATTESTATION_ERROR",
                    "Failed to generate attestation document".to_string(),
                )
            }
        }
    }

    fn handle_seal(&self, data: Vec<u8>) -> EnclaveResponse {
        debug!("Handling Seal request ({} bytes)", data.len());

        match self.key_manager.seal(&data) {
            Ok(sealed_data) => EnclaveResponse::Sealed { sealed_data },
            Err(e) => {
                error!("Seal error: {}", e);
                EnclaveResponse::error("SEAL_ERROR", "Failed to seal data".to_string())
            }
        }
    }

    fn handle_unseal(&self, sealed_data: Vec<u8>) -> EnclaveResponse {
        debug!("Handling Unseal request ({} bytes)", sealed_data.len());

        match self.key_manager.unseal(&sealed_data) {
            Ok(data) => EnclaveResponse::Unsealed { data },
            Err(e) => {
                error!("Unseal error: {}", e);
                EnclaveResponse::error("UNSEAL_ERROR", "Failed to unseal data".to_string())
            }
        }
    }

    fn handle_sign(&self, key_id: String, data: Vec<u8>) -> EnclaveResponse {
        debug!(
            "Handling Sign request (key={}, {} bytes)",
            key_id,
            data.len()
        );

        match self.key_manager.sign(&key_id, &data) {
            Ok((signature, public_key)) => EnclaveResponse::Signature {
                signature,
                public_key,
            },
            Err(e) => {
                error!("Sign error: {}", e);
                EnclaveResponse::error("SIGN_ERROR", "Failed to sign payload".to_string())
            }
        }
    }

    fn handle_generate_key_pair(&self, algorithm: String, label: String) -> EnclaveResponse {
        debug!(
            "Handling GenerateKeyPair request (algo={}, label={})",
            algorithm, label
        );

        match self.key_manager.generate_key_pair(&algorithm, &label) {
            Ok((key_id, public_key)) => EnclaveResponse::KeyGenerated { key_id, public_key },
            Err(e) => {
                error!("GenerateKeyPair error: {}", e);
                EnclaveResponse::error("KEYGEN_ERROR", "Failed to generate key pair".to_string())
            }
        }
    }

    fn handle_get_public_key(&self, key_id: String) -> EnclaveResponse {
        debug!("Handling GetPublicKey request (key={})", key_id);

        match self.key_manager.get_public_key(&key_id) {
            Ok(public_key) => EnclaveResponse::PublicKey { key_id, public_key },
            Err(e) => {
                error!("GetPublicKey error: {}", e);
                EnclaveResponse::error("KEY_NOT_FOUND", "Key not found".to_string())
            }
        }
    }

    fn handle_delete_key(&self, key_id: String) -> EnclaveResponse {
        debug!("Handling DeleteKey request (key={})", key_id);

        match self.key_manager.delete_key(&key_id) {
            Ok(()) => EnclaveResponse::KeyDeleted { key_id },
            Err(e) => {
                error!("DeleteKey error: {}", e);
                EnclaveResponse::error("KEY_NOT_FOUND", "Key not found".to_string())
            }
        }
    }

    fn handle_health_check(&self) -> EnclaveResponse {
        debug!("Handling HealthCheck request");

        EnclaveResponse::Health {
            healthy: true,
            version: VERSION.to_string(),
            uptime_seconds: self.key_manager.uptime_seconds(),
        }
    }

    fn handle_heartbeat(&self) -> EnclaveResponse {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        EnclaveResponse::HeartbeatAck { timestamp }
    }

    fn handle_get_status(&self) -> EnclaveResponse {
        debug!("Handling GetStatus request");

        EnclaveResponse::Status {
            version: VERSION.to_string(),
            key_count: self.key_manager.key_count(),
            memory_used: 0, // Would need to track actual memory usage
            uptime_seconds: self.key_manager.uptime_seconds(),
        }
    }
}

impl Default for RequestHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        let handler = RequestHandler::new();
        let response = handler.handle(EnclaveRequest::HealthCheck);

        match response {
            EnclaveResponse::Health { healthy, .. } => assert!(healthy),
            _ => panic!("Expected Health response"),
        }
    }

    #[test]
    fn test_generate_and_sign() {
        let handler = RequestHandler::new();

        // Generate key
        let response = handler.handle(EnclaveRequest::GenerateKeyPair {
            algorithm: "Ed25519".to_string(),
            label: "test".to_string(),
        });

        let key_id = match response {
            EnclaveResponse::KeyGenerated { key_id, .. } => key_id,
            _ => panic!("Expected KeyGenerated response"),
        };

        // Sign data
        let response = handler.handle(EnclaveRequest::Sign {
            key_id: key_id.clone(),
            data: b"test data".to_vec(),
        });

        match response {
            EnclaveResponse::Signature {
                signature,
                public_key,
            } => {
                assert_eq!(signature.len(), 64);
                assert_eq!(public_key.len(), 32);
            }
            _ => panic!("Expected Signature response"),
        }
    }

    #[test]
    fn test_seal_unseal() {
        let handler = RequestHandler::new();

        let plaintext = b"secret data".to_vec();

        // Seal
        let response = handler.handle(EnclaveRequest::Seal {
            data: plaintext.clone(),
        });
        let sealed_data = match response {
            EnclaveResponse::Sealed { sealed_data } => sealed_data,
            _ => panic!("Expected Sealed response"),
        };

        // Unseal
        let response = handler.handle(EnclaveRequest::Unseal { sealed_data });
        match response {
            EnclaveResponse::Unsealed { data } => {
                assert_eq!(data, plaintext);
            }
            _ => panic!("Expected Unsealed response"),
        }
    }
}
