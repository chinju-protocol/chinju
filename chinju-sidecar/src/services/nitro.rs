//! Nitro Enclave Service
//!
//! Provides a wrapper around chinju-core's Nitro Enclave client for use
//! in the sidecar service. This module handles:
//!
//! - Enclave connection management
//! - Health monitoring
//! - Key operations (sign, seal, unseal)
//! - Attestation retrieval
//!
//! # Configuration
//!
//! Set the following environment variables:
//! - `CHINJU_NITRO_ENABLED=true`: Enable Nitro Enclave support
//! - `CHINJU_NITRO_ENCLAVE_CID`: Enclave CID (from `nitro-cli describe-enclaves`)
//! - `CHINJU_NITRO_PORT`: vsock port (default: 5000)
//! - `CHINJU_NITRO_DEBUG`: Enable debug mode (bypass attestation verification)

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Nitro Enclave service status
#[derive(Debug, Clone)]
pub struct NitroStatus {
    /// Whether the Enclave is enabled
    pub enabled: bool,
    /// Whether the Enclave is connected
    pub connected: bool,
    /// Enclave CID
    pub cid: Option<u32>,
    /// Enclave version
    pub version: Option<String>,
    /// Number of keys in Enclave
    pub key_count: usize,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last error message
    pub last_error: Option<String>,
}

impl Default for NitroStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            connected: false,
            cid: None,
            version: None,
            key_count: 0,
            uptime_seconds: 0,
            last_error: None,
        }
    }
}

/// Nitro Enclave service configuration
#[derive(Debug, Clone)]
pub struct NitroServiceConfig {
    /// Enable Nitro Enclave
    pub enabled: bool,
    /// Enclave CID
    pub cid: Option<u32>,
    /// vsock port
    pub port: u32,
    /// Debug mode (bypass attestation)
    pub debug: bool,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for NitroServiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cid: None,
            port: 5000,
            debug: false,
            timeout_ms: 5000,
        }
    }
}

impl NitroServiceConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("CHINJU_NITRO_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let cid: Option<u32> = std::env::var("CHINJU_NITRO_ENCLAVE_CID")
            .ok()
            .and_then(|s| s.parse().ok());

        let port: u32 = std::env::var("CHINJU_NITRO_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000);

        let debug = std::env::var("CHINJU_NITRO_DEBUG")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let timeout_ms: u64 = std::env::var("CHINJU_NITRO_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000);

        Self {
            enabled,
            cid,
            port,
            debug,
            timeout_ms,
        }
    }
}

/// Nitro Enclave service
///
/// This is a platform-independent wrapper that compiles on all platforms.
/// The actual Nitro functionality is only available on Linux with the
/// `nitro` feature enabled.
pub struct NitroService {
    config: NitroServiceConfig,
    status: Arc<RwLock<NitroStatus>>,
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    client: Option<chinju_core::hardware::NitroEnclaveClient>,
}

impl NitroService {
    /// Create a new Nitro service
    pub fn new(config: NitroServiceConfig) -> Self {
        info!(
            "NitroService created (enabled={}, cid={:?}, port={})",
            config.enabled, config.cid, config.port
        );

        let status = NitroStatus {
            enabled: config.enabled,
            cid: config.cid,
            ..Default::default()
        };

        Self {
            config,
            status: Arc::new(RwLock::new(status)),
            #[cfg(all(feature = "nitro", target_os = "linux"))]
            client: None,
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(NitroServiceConfig::from_env())
    }

    /// Initialize connection to Enclave
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn connect(&mut self) -> Result<(), String> {
        use chinju_core::hardware::nitro::{NitroEnclaveClient, VsockConfig};

        if !self.config.enabled {
            return Ok(());
        }

        let cid = self.config.cid.ok_or("Enclave CID not configured")?;

        info!("Connecting to Nitro Enclave (CID={}, port={})", cid, self.config.port);

        let vsock_config = VsockConfig::new(cid, self.config.port)
            .with_timeout(self.config.timeout_ms);

        match NitroEnclaveClient::new(vsock_config) {
            Ok(client) => {
                // Test connection with health check
                match client.health_check() {
                    Ok((healthy, version, uptime)) => {
                        let mut status = self.status.write().await;
                        status.connected = healthy;
                        status.version = Some(version);
                        status.uptime_seconds = uptime;
                        status.last_error = None;

                        self.client = Some(client);
                        info!("Connected to Nitro Enclave successfully");
                        Ok(())
                    }
                    Err(e) => {
                        let mut status = self.status.write().await;
                        status.connected = false;
                        status.last_error = Some(e.to_string());
                        Err(format!("Enclave health check failed: {}", e))
                    }
                }
            }
            Err(e) => {
                let mut status = self.status.write().await;
                status.connected = false;
                status.last_error = Some(e.to_string());
                Err(format!("Failed to create client: {}", e))
            }
        }
    }

    /// Initialize connection (no-op on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn connect(&mut self) -> Result<(), String> {
        if self.config.enabled {
            warn!("Nitro Enclaves are only supported on Linux with the 'nitro' feature");
            let mut status = self.status.write().await;
            status.last_error = Some("Nitro not available on this platform".to_string());
        }
        Ok(())
    }

    /// Get current status
    pub async fn status(&self) -> NitroStatus {
        self.status.read().await.clone()
    }

    /// Check if Enclave is healthy
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn is_healthy(&self) -> bool {
        if let Some(ref client) = self.client {
            match client.health_check() {
                Ok((healthy, _, _)) => healthy,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Check if Enclave is healthy (always false on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn is_healthy(&self) -> bool {
        false
    }

    /// Send heartbeat to Enclave
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn heartbeat(&self) -> Result<u64, String> {
        if let Some(ref client) = self.client {
            client.heartbeat().map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    /// Send heartbeat (no-op on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn heartbeat(&self) -> Result<u64, String> {
        Err("Nitro not available on this platform".to_string())
    }

    /// Sign data using Enclave key
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn sign(&self, key_id: &str, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        if let Some(ref client) = self.client {
            client.sign(key_id, data).map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    /// Sign data (not available on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn sign(&self, _key_id: &str, _data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        Err("Nitro not available on this platform".to_string())
    }

    /// Seal data using Enclave
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn seal(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref client) = self.client {
            client.seal(data).map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    /// Seal data (not available on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn seal(&self, _data: &[u8]) -> Result<Vec<u8>, String> {
        Err("Nitro not available on this platform".to_string())
    }

    /// Unseal data using Enclave
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn unseal(&self, sealed_data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref client) = self.client {
            client.unseal(sealed_data).map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    /// Unseal data (not available on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn unseal(&self, _sealed_data: &[u8]) -> Result<Vec<u8>, String> {
        Err("Nitro not available on this platform".to_string())
    }

    /// Get attestation document
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn get_attestation(
        &self,
        challenge: &[u8],
        user_data: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        if let Some(ref client) = self.client {
            client.get_attestation(challenge, user_data).map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    /// Get attestation document (not available on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn get_attestation(
        &self,
        _challenge: &[u8],
        _user_data: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        Err("Nitro not available on this platform".to_string())
    }

    /// Generate key pair in Enclave
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    pub async fn generate_key_pair(
        &self,
        algorithm: &str,
        label: &str,
    ) -> Result<(String, Vec<u8>), String> {
        if let Some(ref client) = self.client {
            client.generate_key_pair(algorithm, label).map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    /// Generate key pair (not available on non-Linux)
    #[cfg(not(all(feature = "nitro", target_os = "linux")))]
    pub async fn generate_key_pair(
        &self,
        _algorithm: &str,
        _label: &str,
    ) -> Result<(String, Vec<u8>), String> {
        Err("Nitro not available on this platform".to_string())
    }

    /// Get config
    pub fn config(&self) -> &NitroServiceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = NitroServiceConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 5000);
    }

    #[tokio::test]
    async fn test_service_disabled() {
        let service = NitroService::new(NitroServiceConfig::default());
        let status = service.status().await;
        assert!(!status.enabled);
        assert!(!status.connected);
    }
}
