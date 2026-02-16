//! KMS Client for Nitro Enclave
//!
//! This module provides KMS access from inside the Nitro Enclave via vsock-proxy.
//!
//! # Architecture
//!
//! ```text
//! Nitro Enclave                 EC2 Parent                     AWS
//! +------------------+          +------------------+           +-----+
//! | chinju-enclave   |  vsock   | vsock-proxy      |  HTTPS    | KMS |
//! | KmsClient        | -------> | (CID 3)          | --------> |     |
//! +------------------+          +------------------+           +-----+
//!       |                             |                            |
//!       +-- Attestation Document --------------------------------->|
//! ```
//!
//! # vsock-proxy Setup (Parent Instance)
//!
//! ```bash
//! # Start vsock-proxy for KMS
//! vsock-proxy 8000 kms.ap-northeast-1.amazonaws.com 443
//! ```
//!
//! # KMS Key Policy for Enclave
//!
//! The KMS key must have a policy that allows access from Enclaves with specific PCR values:
//!
//! ```json
//! {
//!   "Version": "2012-10-17",
//!   "Statement": [
//!     {
//!       "Sid": "AllowEnclaveDecrypt",
//!       "Effect": "Allow",
//!       "Principal": { "AWS": "arn:aws:iam::123456789012:role/EnclaveRole" },
//!       "Action": ["kms:Decrypt", "kms:GenerateDataKey"],
//!       "Resource": "*",
//!       "Condition": {
//!         "StringEquals": {
//!           "kms:RecipientAttestation:PCR0": "<enclave_pcr0_hash>"
//!         }
//!       }
//!     }
//!   ]
//! }
//! ```

use crate::nsm::NsmClient;
use crate::EnclaveError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use tracing::{debug, info, warn};

/// KMS Client Configuration
#[derive(Debug, Clone)]
pub struct KmsClientConfig {
    /// vsock-proxy CID (typically 3 for parent)
    pub vsock_proxy_cid: u32,
    /// vsock-proxy port for KMS
    pub vsock_proxy_port: u32,
    /// AWS Region
    pub region: String,
    /// KMS Key ID or ARN
    pub key_id: String,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

impl Default for KmsClientConfig {
    fn default() -> Self {
        Self {
            vsock_proxy_cid: 3, // Parent CID
            vsock_proxy_port: 8000,
            region: "us-east-1".to_string(),
            key_id: String::new(),
            timeout_secs: 30,
        }
    }
}

impl KmsClientConfig {
    /// Load from environment variables
    pub fn from_env() -> Result<Self, EnclaveError> {
        let key_id = std::env::var("AWS_KMS_KEY_ID")
            .map_err(|_| EnclaveError::InvalidRequest("AWS_KMS_KEY_ID not set".to_string()))?;

        let region = std::env::var("AWS_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string());

        let vsock_proxy_cid: u32 = std::env::var("VSOCK_PROXY_CID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        let vsock_proxy_port: u32 = std::env::var("VSOCK_PROXY_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8000);

        Ok(Self {
            vsock_proxy_cid,
            vsock_proxy_port,
            region,
            key_id,
            ..Default::default()
        })
    }
}

/// KMS Encrypt Request
#[derive(Debug, Serialize)]
struct KmsEncryptRequest {
    #[serde(rename = "KeyId")]
    key_id: String,
    #[serde(rename = "Plaintext")]
    plaintext: String, // Base64 encoded
    #[serde(rename = "EncryptionContext", skip_serializing_if = "Option::is_none")]
    encryption_context: Option<HashMap<String, String>>,
}

/// KMS Encrypt Response
#[derive(Debug, Deserialize)]
struct KmsEncryptResponse {
    #[serde(rename = "CiphertextBlob")]
    ciphertext_blob: String, // Base64 encoded
    #[serde(rename = "KeyId")]
    key_id: String,
}

/// KMS Decrypt Request
#[derive(Debug, Serialize)]
struct KmsDecryptRequest {
    #[serde(rename = "CiphertextBlob")]
    ciphertext_blob: String, // Base64 encoded
    #[serde(rename = "KeyId", skip_serializing_if = "Option::is_none")]
    key_id: Option<String>,
    #[serde(rename = "EncryptionContext", skip_serializing_if = "Option::is_none")]
    encryption_context: Option<HashMap<String, String>>,
    #[serde(rename = "Recipient", skip_serializing_if = "Option::is_none")]
    recipient: Option<KmsRecipient>,
}

/// KMS Recipient (for Attestation)
#[derive(Debug, Serialize)]
struct KmsRecipient {
    #[serde(rename = "AttestationDocument")]
    attestation_document: String, // Base64 encoded CBOR
    #[serde(rename = "KeyEncryptionAlgorithm")]
    key_encryption_algorithm: String, // "RSAES_OAEP_SHA_256"
}

/// KMS Decrypt Response
#[derive(Debug, Deserialize)]
struct KmsDecryptResponse {
    #[serde(rename = "Plaintext")]
    plaintext: Option<String>, // Base64 encoded (None if using Recipient)
    #[serde(rename = "CiphertextForRecipient")]
    ciphertext_for_recipient: Option<String>, // Base64 encoded (when using Recipient)
    #[serde(rename = "KeyId")]
    key_id: String,
}

/// KMS GenerateDataKey Request
#[derive(Debug, Serialize)]
struct KmsGenerateDataKeyRequest {
    #[serde(rename = "KeyId")]
    key_id: String,
    #[serde(rename = "KeySpec")]
    key_spec: String, // "AES_256"
    #[serde(rename = "EncryptionContext", skip_serializing_if = "Option::is_none")]
    encryption_context: Option<HashMap<String, String>>,
    #[serde(rename = "Recipient", skip_serializing_if = "Option::is_none")]
    recipient: Option<KmsRecipient>,
}

/// KMS GenerateDataKey Response
#[derive(Debug, Deserialize)]
struct KmsGenerateDataKeyResponse {
    #[serde(rename = "Plaintext")]
    plaintext: Option<String>, // Base64 encoded (None if using Recipient)
    #[serde(rename = "CiphertextBlob")]
    ciphertext_blob: String, // Base64 encoded
    #[serde(rename = "CiphertextForRecipient")]
    ciphertext_for_recipient: Option<String>, // Base64 encoded (when using Recipient)
    #[serde(rename = "KeyId")]
    key_id: String,
}

/// KMS Error Response
#[derive(Debug, Deserialize)]
struct KmsErrorResponse {
    #[serde(rename = "__type")]
    error_type: String,
    message: Option<String>,
}

/// KMS Client for Nitro Enclave
///
/// Communicates with AWS KMS via vsock-proxy.
pub struct KmsClient {
    config: KmsClientConfig,
    nsm: NsmClient,
}

impl KmsClient {
    /// Create a new KMS client
    pub fn new(config: KmsClientConfig) -> Self {
        info!(
            "KmsClient initialized (region={}, key_id={}, proxy={}:{})",
            config.region, config.key_id, config.vsock_proxy_cid, config.vsock_proxy_port
        );

        Self {
            config,
            nsm: NsmClient::default(),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, EnclaveError> {
        Ok(Self::new(KmsClientConfig::from_env()?))
    }

    /// Connect to vsock-proxy
    fn connect(&self) -> Result<std::fs::File, EnclaveError> {
        use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, VsockAddr};

        debug!(
            "Connecting to vsock-proxy at CID:{} port:{}",
            self.config.vsock_proxy_cid, self.config.vsock_proxy_port
        );

        let socket_fd = socket(
            AddressFamily::Vsock,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .map_err(|e| EnclaveError::SocketError(format!("Failed to create socket: {}", e)))?;

        let addr = VsockAddr::new(self.config.vsock_proxy_cid, self.config.vsock_proxy_port);

        connect(socket_fd.as_raw_fd(), &addr)
            .map_err(|e| EnclaveError::SocketError(format!("Failed to connect: {}", e)))?;

        // Convert to File for Read/Write traits
        let stream = unsafe { std::fs::File::from_raw_fd(socket_fd.into_raw_fd()) };

        Ok(stream)
    }

    /// Send HTTP request to KMS via vsock-proxy
    fn send_request(&self, action: &str, body: &str) -> Result<String, EnclaveError> {
        let mut stream = self.connect()?;

        // Build HTTP request
        let host = format!("kms.{}.amazonaws.com", self.config.region);
        let http_request = format!(
            "POST / HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/x-amz-json-1.1\r\n\
             X-Amz-Target: TrentService.{}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            host,
            action,
            body.len(),
            body
        );

        debug!("Sending KMS request: {}", action);

        // Send request
        stream
            .write_all(http_request.as_bytes())
            .map_err(|e| EnclaveError::IoError(e))?;

        stream.flush().map_err(|e| EnclaveError::IoError(e))?;

        // Read response
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|e| EnclaveError::IoError(e))?;

        let response_str = String::from_utf8_lossy(&response);

        // Parse HTTP response
        let parts: Vec<&str> = response_str.splitn(2, "\r\n\r\n").collect();
        if parts.len() != 2 {
            return Err(EnclaveError::SerializationError(
                "Invalid HTTP response".to_string(),
            ));
        }

        let headers = parts[0];
        let body = parts[1];

        // Check for HTTP errors
        if !headers.contains("200 OK") {
            // Try to parse KMS error
            if let Ok(error) = serde_json::from_str::<KmsErrorResponse>(body) {
                return Err(EnclaveError::NsmError(format!(
                    "KMS error: {} - {}",
                    error.error_type,
                    error.message.unwrap_or_default()
                )));
            }
            return Err(EnclaveError::NsmError(format!(
                "KMS request failed: {}",
                headers.lines().next().unwrap_or("Unknown error")
            )));
        }

        Ok(body.to_string())
    }

    /// Encrypt data using KMS
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EnclaveError> {
        let request = KmsEncryptRequest {
            key_id: self.config.key_id.clone(),
            plaintext: base64_encode(plaintext),
            encryption_context: None,
        };

        let body = serde_json::to_string(&request)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let response_body = self.send_request("Encrypt", &body)?;

        let response: KmsEncryptResponse = serde_json::from_str(&response_body)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let ciphertext = base64_decode(&response.ciphertext_blob)?;

        debug!("KMS Encrypt: {} bytes -> {} bytes", plaintext.len(), ciphertext.len());

        Ok(ciphertext)
    }

    /// Decrypt data using KMS
    ///
    /// When using attestation-based access control, the KMS key policy
    /// will verify the Enclave's PCR values before allowing decryption.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, EnclaveError> {
        let request = KmsDecryptRequest {
            ciphertext_blob: base64_encode(ciphertext),
            key_id: Some(self.config.key_id.clone()),
            encryption_context: None,
            recipient: None, // Standard decrypt without attestation
        };

        let body = serde_json::to_string(&request)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let response_body = self.send_request("Decrypt", &body)?;

        let response: KmsDecryptResponse = serde_json::from_str(&response_body)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let plaintext = response
            .plaintext
            .ok_or_else(|| EnclaveError::NsmError("No plaintext in response".to_string()))?;

        let data = base64_decode(&plaintext)?;

        debug!("KMS Decrypt: {} bytes -> {} bytes", ciphertext.len(), data.len());

        Ok(data)
    }

    /// Decrypt data using KMS with Attestation
    ///
    /// This method includes an attestation document in the request.
    /// KMS will:
    /// 1. Verify the attestation document signature
    /// 2. Check PCR values against the key policy
    /// 3. Return the data key encrypted to the Enclave's public key
    pub fn decrypt_with_attestation(
        &self,
        ciphertext: &[u8],
        user_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, EnclaveError> {
        // Get attestation document from NSM
        let nonce: [u8; 32] = rand::random();
        let attestation_doc = self.nsm.get_attestation_document(
            user_data,
            Some(&nonce),
            None, // public_key will be extracted from attestation
        )?;

        let request = KmsDecryptRequest {
            ciphertext_blob: base64_encode(ciphertext),
            key_id: Some(self.config.key_id.clone()),
            encryption_context: None,
            recipient: Some(KmsRecipient {
                attestation_document: base64_encode(&attestation_doc),
                key_encryption_algorithm: "RSAES_OAEP_SHA_256".to_string(),
            }),
        };

        let body = serde_json::to_string(&request)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let response_body = self.send_request("Decrypt", &body)?;

        let response: KmsDecryptResponse = serde_json::from_str(&response_body)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        // When using Recipient, the response contains CiphertextForRecipient
        // which is encrypted with the Enclave's public key
        let ciphertext_for_recipient = response
            .ciphertext_for_recipient
            .ok_or_else(|| EnclaveError::NsmError("No ciphertext for recipient".to_string()))?;

        let encrypted_data = base64_decode(&ciphertext_for_recipient)?;

        // Decrypt using NSM private key (in production)
        // For now, return the encrypted data - real implementation would use NSM to decrypt
        warn!("decrypt_with_attestation: NSM decryption not implemented, returning encrypted data");

        Ok(encrypted_data)
    }

    /// Generate a data key using KMS with Attestation
    ///
    /// Returns (plaintext_key, encrypted_key) where:
    /// - plaintext_key: Data key for immediate use (encrypted to Enclave's public key)
    /// - encrypted_key: Data key encrypted with KMS key (for storage)
    pub fn generate_data_key_with_attestation(
        &self,
        user_data: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), EnclaveError> {
        // Get attestation document from NSM
        let nonce: [u8; 32] = rand::random();
        let attestation_doc = self.nsm.get_attestation_document(
            user_data,
            Some(&nonce),
            None,
        )?;

        let request = KmsGenerateDataKeyRequest {
            key_id: self.config.key_id.clone(),
            key_spec: "AES_256".to_string(),
            encryption_context: None,
            recipient: Some(KmsRecipient {
                attestation_document: base64_encode(&attestation_doc),
                key_encryption_algorithm: "RSAES_OAEP_SHA_256".to_string(),
            }),
        };

        let body = serde_json::to_string(&request)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let response_body = self.send_request("GenerateDataKey", &body)?;

        let response: KmsGenerateDataKeyResponse = serde_json::from_str(&response_body)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let encrypted_key = base64_decode(&response.ciphertext_blob)?;

        // When using Recipient, plaintext is encrypted to Enclave
        let ciphertext_for_recipient = response
            .ciphertext_for_recipient
            .ok_or_else(|| EnclaveError::NsmError("No ciphertext for recipient".to_string()))?;

        let encrypted_plaintext = base64_decode(&ciphertext_for_recipient)?;

        debug!(
            "KMS GenerateDataKey: encrypted_key={} bytes, encrypted_plaintext={} bytes",
            encrypted_key.len(),
            encrypted_plaintext.len()
        );

        Ok((encrypted_plaintext, encrypted_key))
    }

    /// Generate a data key (without attestation)
    pub fn generate_data_key(&self) -> Result<(Vec<u8>, Vec<u8>), EnclaveError> {
        let request = KmsGenerateDataKeyRequest {
            key_id: self.config.key_id.clone(),
            key_spec: "AES_256".to_string(),
            encryption_context: None,
            recipient: None,
        };

        let body = serde_json::to_string(&request)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let response_body = self.send_request("GenerateDataKey", &body)?;

        let response: KmsGenerateDataKeyResponse = serde_json::from_str(&response_body)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        let encrypted_key = base64_decode(&response.ciphertext_blob)?;
        let plaintext = response
            .plaintext
            .ok_or_else(|| EnclaveError::NsmError("No plaintext in response".to_string()))?;
        let plaintext_key = base64_decode(&plaintext)?;

        debug!(
            "KMS GenerateDataKey: plaintext={} bytes, encrypted={} bytes",
            plaintext_key.len(),
            encrypted_key.len()
        );

        Ok((plaintext_key, encrypted_key))
    }
}

/// Base64 encode
fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder = base64::write::EncoderWriter::new(&mut buf, &base64::engine::general_purpose::STANDARD);
        encoder.write_all(data).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

/// Base64 decode
fn base64_decode(s: &str) -> Result<Vec<u8>, EnclaveError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| EnclaveError::SerializationError(format!("Base64 decode error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = KmsClientConfig::default();
        assert_eq!(config.vsock_proxy_cid, 3);
        assert_eq!(config.vsock_proxy_port, 8000);
        assert_eq!(config.region, "us-east-1");
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"Hello, KMS!";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
