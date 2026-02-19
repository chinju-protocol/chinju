//! vsock communication wrapper
//!
//! Provides communication between EC2 parent instance and Nitro Enclave
//! using the vsock (virtio socket) protocol.

use super::error::NitroError;
use super::protocol::{EnclaveRequest, EnclaveResponse};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Read, Write};
use std::time::Duration;
use tracing::{debug, trace};
use vsock::{VsockAddr, VsockStream};

/// vsock connection configuration
#[derive(Debug, Clone)]
pub struct VsockConfig {
    /// Enclave Context ID (CID)
    /// Use `nitro-cli describe-enclaves` to get the CID
    pub cid: u32,
    /// vsock port number
    pub port: u32,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
}

impl VsockConfig {
    /// Create a new configuration
    pub fn new(cid: u32, port: u32) -> Self {
        Self {
            cid,
            port,
            timeout_ms: 5000,
        }
    }

    /// Create configuration from environment variables
    ///
    /// Required:
    /// - `CHINJU_NITRO_ENCLAVE_CID`: Enclave CID
    ///
    /// Optional:
    /// - `CHINJU_NITRO_PORT`: vsock port (default: 5000)
    /// - `CHINJU_NITRO_TIMEOUT_MS`: Connection timeout (default: 5000)
    pub fn from_env() -> Result<Self, NitroError> {
        let cid: u32 = std::env::var("CHINJU_NITRO_ENCLAVE_CID")
            .map_err(|_| NitroError::ConfigMissing("CHINJU_NITRO_ENCLAVE_CID"))?
            .parse()
            .map_err(|_| NitroError::InvalidConfig("Invalid CHINJU_NITRO_ENCLAVE_CID"))?;

        let port: u32 = std::env::var("CHINJU_NITRO_PORT")
            .unwrap_or_else(|_| "5000".to_string())
            .parse()
            .map_err(|_| NitroError::InvalidConfig("Invalid CHINJU_NITRO_PORT"))?;

        let timeout_ms: u64 = std::env::var("CHINJU_NITRO_TIMEOUT_MS")
            .unwrap_or_else(|_| "5000".to_string())
            .parse()
            .map_err(|_| NitroError::InvalidConfig("Invalid CHINJU_NITRO_TIMEOUT_MS"))?;

        Ok(Self {
            cid,
            port,
            timeout_ms,
        })
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

impl Default for VsockConfig {
    fn default() -> Self {
        Self {
            cid: 16, // Default Enclave CID
            port: 5000,
            timeout_ms: 5000,
        }
    }
}

/// vsock client for communicating with Nitro Enclave
pub struct VsockClient {
    config: VsockConfig,
}

impl VsockClient {
    /// Create a new vsock client
    pub fn new(config: VsockConfig) -> Self {
        Self { config }
    }

    /// Create a client from environment variables
    pub fn from_env() -> Result<Self, NitroError> {
        Ok(Self::new(VsockConfig::from_env()?))
    }

    /// Get the configuration
    pub fn config(&self) -> &VsockConfig {
        &self.config
    }

    /// Send a request to the Enclave and receive a response
    pub fn request<Req, Res>(&self, request: &Req) -> Result<Res, NitroError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let addr = VsockAddr::new(self.config.cid, self.config.port);
        debug!(
            "Connecting to Enclave at CID:{} port:{}",
            self.config.cid, self.config.port
        );

        let mut stream =
            VsockStream::connect(&addr).map_err(|e| NitroError::ConnectionFailed(e.to_string()))?;

        // Set read/write timeout
        let timeout = Duration::from_millis(self.config.timeout_ms);
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| NitroError::IoError(e.to_string()))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| NitroError::IoError(e.to_string()))?;

        // Serialize request to CBOR
        let request_bytes = serde_cbor::to_vec(request)
            .map_err(|e| NitroError::SerializationFailed(e.to_string()))?;

        trace!("Sending {} bytes to Enclave", request_bytes.len());

        // Send length prefix (4 bytes, big-endian)
        let len = (request_bytes.len() as u32).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(&request_bytes)?;
        stream.flush()?;

        // Receive response length
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let response_len = u32::from_be_bytes(len_buf) as usize;

        trace!("Receiving {} bytes from Enclave", response_len);

        // Receive response body
        let mut response_bytes = vec![0u8; response_len];
        stream.read_exact(&mut response_bytes)?;

        // Deserialize response
        serde_cbor::from_slice(&response_bytes)
            .map_err(|e| NitroError::DeserializationFailed(e.to_string()))
    }

    /// Send a typed EnclaveRequest and receive EnclaveResponse
    pub fn send(&self, request: EnclaveRequest) -> Result<EnclaveResponse, NitroError> {
        let response: EnclaveResponse = self.request(&request)?;

        // Check for error response
        if let EnclaveResponse::Error { code, message } = &response {
            return Err(NitroError::EnclaveError {
                code: code.clone(),
                message: message.clone(),
            });
        }

        Ok(response)
    }

    /// Check if Enclave is available and healthy
    pub fn health_check(&self) -> Result<bool, NitroError> {
        let response = self.send(EnclaveRequest::HealthCheck)?;
        match response {
            EnclaveResponse::Health { healthy, .. } => Ok(healthy),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// Send heartbeat to Enclave
    pub fn heartbeat(&self) -> Result<u64, NitroError> {
        let response = self.send(EnclaveRequest::Heartbeat)?;
        match response {
            EnclaveResponse::HeartbeatAck { timestamp } => Ok(timestamp),
            _ => Err(NitroError::CommunicationError(
                "Unexpected response type".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vsock_config_default() {
        let config = VsockConfig::default();
        assert_eq!(config.cid, 16);
        assert_eq!(config.port, 5000);
        assert_eq!(config.timeout_ms, 5000);
    }

    #[test]
    fn test_vsock_config_new() {
        let config = VsockConfig::new(20, 6000);
        assert_eq!(config.cid, 20);
        assert_eq!(config.port, 6000);
    }

    #[test]
    fn test_vsock_config_with_timeout() {
        let config = VsockConfig::default().with_timeout(10000);
        assert_eq!(config.timeout_ms, 10000);
    }
}
