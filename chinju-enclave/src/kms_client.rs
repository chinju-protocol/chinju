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
use hmac::{Hmac, Mac};
use rsa::pkcs8::EncodePublicKey;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use tracing::{debug, info, warn};

type HmacSha256 = Hmac<Sha256>;

/// AWS credentials for request signing
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

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
    /// Allow unsigned direct KMS requests (unsafe, development only)
    pub allow_insecure_unsigned_requests: bool,
    /// AWS credentials (required for signed mode)
    pub credentials: Option<AwsCredentials>,
}

impl Default for KmsClientConfig {
    fn default() -> Self {
        Self {
            vsock_proxy_cid: 3, // Parent CID
            vsock_proxy_port: 8000,
            region: "us-east-1".to_string(),
            key_id: String::new(),
            timeout_secs: 30,
            allow_insecure_unsigned_requests: false,
            credentials: None,
        }
    }
}

impl KmsClientConfig {
    /// Load from environment variables
    pub fn from_env() -> Result<Self, EnclaveError> {
        let key_id = std::env::var("AWS_KMS_KEY_ID")
            .map_err(|_| EnclaveError::InvalidRequest("AWS_KMS_KEY_ID not set".to_string()))?;

        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let vsock_proxy_cid: u32 = std::env::var("VSOCK_PROXY_CID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        let vsock_proxy_port: u32 = std::env::var("VSOCK_PROXY_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8000);

        let allow_insecure_unsigned_requests = std::env::var("CHINJU_KMS_ALLOW_UNSIGNED")
            .ok()
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
        let credentials = match (access_key_id, secret_access_key) {
            (Some(access_key_id), Some(secret_access_key)) => Some(AwsCredentials {
                access_key_id,
                secret_access_key,
                session_token,
            }),
            _ => None,
        };

        Ok(Self {
            vsock_proxy_cid,
            vsock_proxy_port,
            region,
            key_id,
            allow_insecure_unsigned_requests,
            credentials,
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
        if config.allow_insecure_unsigned_requests {
            warn!("KMS client running in UNSAFE unsigned-request mode");
        }

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
        let host = format!("kms.{}.amazonaws.com", self.config.region);

        let http_request = if self.config.allow_insecure_unsigned_requests {
            warn!(
                "KMS request sent without SigV4 signing (development mode only): {}",
                action
            );
            self.build_unsigned_http_request(&host, action, body)
        } else {
            self.build_signed_http_request(&host, action, body)?
        };

        debug!("Sending KMS request: {}", action);

        stream
            .write_all(http_request.as_bytes())
            .map_err(EnclaveError::IoError)?;
        stream.flush().map_err(EnclaveError::IoError)?;
        self.read_http_response(&mut stream)
    }

    fn build_unsigned_http_request(&self, host: &str, action: &str, body: &str) -> String {
        format!(
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
        )
    }

    fn build_signed_http_request(
        &self,
        host: &str,
        action: &str,
        body: &str,
    ) -> Result<String, EnclaveError> {
        let credentials = self.config.credentials.as_ref().ok_or_else(|| {
            EnclaveError::InvalidRequest(
                "Missing AWS credentials for KMS SigV4 signing (AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY)"
                    .to_string(),
            )
        })?;

        let (amz_date, date_stamp) = current_amz_timestamp()?;
        let payload_hash = sha256_hex(body.as_bytes());
        let target = format!("TrentService.{}", action);

        let mut canonical_headers = vec![
            (
                "content-type".to_string(),
                "application/x-amz-json-1.1".to_string(),
            ),
            ("host".to_string(), host.to_string()),
            ("x-amz-content-sha256".to_string(), payload_hash.clone()),
            ("x-amz-date".to_string(), amz_date.clone()),
            ("x-amz-target".to_string(), target.clone()),
        ];
        if let Some(token) = &credentials.session_token {
            canonical_headers.push(("x-amz-security-token".to_string(), token.clone()));
        }
        canonical_headers.sort_by(|a, b| a.0.cmp(&b.0));

        let canonical_headers_str = canonical_headers
            .iter()
            .map(|(k, v)| format!("{}:{}\n", k, v.trim()))
            .collect::<String>();
        let signed_headers = canonical_headers
            .iter()
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>()
            .join(";");

        let canonical_request = format!(
            "POST\n/\n\n{}\n{}\n{}",
            canonical_headers_str, signed_headers, payload_hash
        );
        let credential_scope = format!("{}/{}/kms/aws4_request", date_stamp, self.config.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date,
            credential_scope,
            sha256_hex(canonical_request.as_bytes())
        );
        let signature = compute_sigv4_signature(
            &credentials.secret_access_key,
            &date_stamp,
            &self.config.region,
            "kms",
            &string_to_sign,
        )?;
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            credentials.access_key_id, credential_scope, signed_headers, signature
        );

        let mut request = format!(
            "POST / HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/x-amz-json-1.1\r\n\
             X-Amz-Target: {}\r\n\
             X-Amz-Date: {}\r\n\
             X-Amz-Content-Sha256: {}\r\n\
             Authorization: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n",
            host,
            target,
            amz_date,
            payload_hash,
            authorization,
            body.len()
        );
        if let Some(token) = &credentials.session_token {
            request.push_str(&format!("X-Amz-Security-Token: {}\r\n", token));
        }
        request.push_str("\r\n");
        request.push_str(body);

        Ok(request)
    }

    fn read_http_response(&self, stream: &mut std::fs::File) -> Result<String, EnclaveError> {
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(EnclaveError::IoError)?;

        let response_str = String::from_utf8_lossy(&response);
        let parts: Vec<&str> = response_str.splitn(2, "\r\n\r\n").collect();
        if parts.len() != 2 {
            return Err(EnclaveError::SerializationError(
                "Invalid HTTP response".to_string(),
            ));
        }

        let headers = parts[0];
        let body = parts[1];

        if !headers.contains("200 OK") {
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

        debug!(
            "KMS Encrypt: {} bytes -> {} bytes",
            plaintext.len(),
            ciphertext.len()
        );

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

        debug!(
            "KMS Decrypt: {} bytes -> {} bytes",
            ciphertext.len(),
            data.len()
        );

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
        let (recipient, private_key) = self.build_attested_recipient(user_data)?;

        let request = KmsDecryptRequest {
            ciphertext_blob: base64_encode(ciphertext),
            key_id: Some(self.config.key_id.clone()),
            encryption_context: None,
            recipient: Some(recipient),
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
        let plaintext = self.decrypt_recipient_ciphertext(&private_key, &encrypted_data)?;
        Ok(plaintext)
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
        let (recipient, private_key) = self.build_attested_recipient(user_data)?;

        let request = KmsGenerateDataKeyRequest {
            key_id: self.config.key_id.clone(),
            key_spec: "AES_256".to_string(),
            encryption_context: None,
            recipient: Some(recipient),
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
        let plaintext_key =
            self.decrypt_recipient_ciphertext(&private_key, &encrypted_plaintext)?;

        debug!(
            "KMS GenerateDataKey: encrypted_key={} bytes, plaintext_key={} bytes",
            encrypted_key.len(),
            plaintext_key.len()
        );

        Ok((plaintext_key, encrypted_key))
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

    fn build_attested_recipient(
        &self,
        user_data: Option<&[u8]>,
    ) -> Result<(KmsRecipient, RsaPrivateKey), EnclaveError> {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).map_err(|e| {
            EnclaveError::NsmError(format!("Failed to generate recipient RSA keypair: {}", e))
        })?;
        let public_key = RsaPublicKey::from(&private_key);
        let public_key_der = public_key.to_public_key_der().map_err(|e| {
            EnclaveError::SerializationError(format!(
                "Failed to encode recipient public key (DER): {}",
                e
            ))
        })?;

        let nonce: [u8; 32] = rand::random();
        let attestation_doc = self.nsm.get_attestation_document(
            user_data,
            Some(&nonce),
            Some(public_key_der.as_ref()),
        )?;

        Ok((
            KmsRecipient {
                attestation_document: base64_encode(&attestation_doc),
                key_encryption_algorithm: "RSAES_OAEP_SHA_256".to_string(),
            },
            private_key,
        ))
    }

    fn decrypt_recipient_ciphertext(
        &self,
        private_key: &RsaPrivateKey,
        encrypted_data: &[u8],
    ) -> Result<Vec<u8>, EnclaveError> {
        let padding = Oaep::new::<Sha256>();
        private_key
            .decrypt(padding, encrypted_data)
            .map_err(|e| EnclaveError::NsmError(format!("Recipient decrypt failed: {}", e)))
    }
}

fn current_amz_timestamp() -> Result<(String, String), EnclaveError> {
    use chrono::{Datelike, Timelike, Utc};

    let now = Utc::now();
    let amz_date = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let date_stamp = format!("{:04}{:02}{:02}", now.year(), now.month(), now.day());
    Ok((amz_date, date_stamp))
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, EnclaveError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| {
        EnclaveError::SerializationError(format!("Failed to initialize HMAC: {}", e))
    })?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn compute_sigv4_signature(
    secret_access_key: &str,
    date_stamp: &str,
    region: &str,
    service: &str,
    string_to_sign: &str,
) -> Result<String, EnclaveError> {
    let k_secret = format!("AWS4{}", secret_access_key);
    let k_date = hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, service.as_bytes())?;
    let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
    let signature = hmac_sha256(&k_signing, string_to_sign.as_bytes())?;
    Ok(hex::encode(signature))
}

/// Base64 encode
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
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
