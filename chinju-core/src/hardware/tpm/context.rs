//! TPM context management

use std::convert::TryFrom;
use thiserror::Error;
use tss_esapi::{
    abstraction::transient::TransientKeyContextBuilder,
    tcti_ldr::{TctiNameConf, TabrmdConfig, DeviceConfig, SwtpmConfig},
    Context,
};
use tracing::{debug, info, warn};

/// TPM-related errors
#[derive(Debug, Error)]
pub enum TpmError {
    #[error("TPM initialization failed: {0}")]
    InitializationFailed(String),

    #[error("TPM communication error: {0}")]
    CommunicationError(String),

    #[error("TPM operation failed: {0}")]
    OperationFailed(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("PCR error: {0}")]
    PcrError(String),

    #[error("Attestation failed: {0}")]
    AttestationFailed(String),

    #[error("Sealing/Unsealing error: {0}")]
    SealingError(String),

    #[error("Not supported: {0}")]
    NotSupported(String),
}

impl From<tss_esapi::Error> for TpmError {
    fn from(e: tss_esapi::Error) -> Self {
        TpmError::OperationFailed(e.to_string())
    }
}

/// TPM interface type
#[derive(Debug, Clone)]
pub enum TpmInterface {
    /// TCP socket connection (for swtpm)
    Socket { host: String, port: u16 },
    /// Device file (hardware TPM)
    Device { path: String },
    /// D-Bus connection (tpm2-abrmd)
    Tabrmd,
}

impl Default for TpmInterface {
    fn default() -> Self {
        // Default to socket for swtpm development
        TpmInterface::Socket {
            host: "localhost".to_string(),
            port: 2321,
        }
    }
}

/// TPM configuration
#[derive(Debug, Clone)]
pub struct TpmConfig {
    /// Interface type
    pub interface: TpmInterface,
    /// Session encryption enabled
    pub encrypt_sessions: bool,
}

impl Default for TpmConfig {
    fn default() -> Self {
        Self {
            interface: TpmInterface::default(),
            encrypt_sessions: true,
        }
    }
}

impl TpmConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        let interface = match std::env::var("TPM_INTERFACE")
            .unwrap_or_else(|_| "socket".to_string())
            .to_lowercase()
            .as_str()
        {
            "device" => {
                let path = std::env::var("TPM_DEVICE")
                    .unwrap_or_else(|_| "/dev/tpm0".to_string());
                TpmInterface::Device { path }
            }
            "tabrmd" => TpmInterface::Tabrmd,
            _ => {
                let host = std::env::var("TPM_HOST")
                    .unwrap_or_else(|_| "localhost".to_string());
                let port: u16 = std::env::var("TPM_PORT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(2321);
                TpmInterface::Socket { host, port }
            }
        };

        Self {
            interface,
            encrypt_sessions: true,
        }
    }

    /// Create socket configuration
    pub fn socket(host: impl Into<String>, port: u16) -> Self {
        Self {
            interface: TpmInterface::Socket {
                host: host.into(),
                port,
            },
            encrypt_sessions: true,
        }
    }

    /// Create device configuration
    pub fn device(path: impl Into<String>) -> Self {
        Self {
            interface: TpmInterface::Device { path: path.into() },
            encrypt_sessions: true,
        }
    }

    /// Get TCTI name configuration
    fn to_tcti_name_conf(&self) -> Result<TctiNameConf, TpmError> {
        match &self.interface {
            TpmInterface::Socket { host, port } => {
                let config = SwtpmConfig::from_str(&format!("host={},port={}", host, port))
                    .map_err(|e| TpmError::InitializationFailed(e.to_string()))?;
                Ok(TctiNameConf::Swtpm(config))
            }
            TpmInterface::Device { path } => {
                let config = DeviceConfig::from_str(path)
                    .map_err(|e| TpmError::InitializationFailed(e.to_string()))?;
                Ok(TctiNameConf::Device(config))
            }
            TpmInterface::Tabrmd => {
                Ok(TctiNameConf::Tabrmd(TabrmdConfig::default()))
            }
        }
    }
}

/// TPM context wrapper
pub struct TpmContext {
    context: Context,
    config: TpmConfig,
}

impl TpmContext {
    /// Create a new TPM context
    pub fn new(config: TpmConfig) -> Result<Self, TpmError> {
        info!("Initializing TPM context with {:?}", config.interface);

        let tcti = config.to_tcti_name_conf()?;
        let context = Context::new(tcti)
            .map_err(|e| TpmError::InitializationFailed(e.to_string()))?;

        debug!("TPM context created successfully");

        Ok(Self { context, config })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, TpmError> {
        Self::new(TpmConfig::from_env())
    }

    /// Get the underlying context
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Get mutable context
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Get configuration
    pub fn config(&self) -> &TpmConfig {
        &self.config
    }

    /// Get TPM manufacturer information
    pub fn get_manufacturer_info(&mut self) -> Result<TpmInfo, TpmError> {
        use tss_esapi::constants::tss::TPM2_PT_MANUFACTURER;
        use tss_esapi::structures::CapabilityData;

        let (caps, _) = self.context
            .get_capability(
                tss_esapi::constants::CapabilityType::TpmProperties,
                TPM2_PT_MANUFACTURER,
                1,
            )
            .map_err(|e| TpmError::OperationFailed(e.to_string()))?;

        let manufacturer = match caps {
            CapabilityData::TpmProperties(props) => {
                props.first()
                    .map(|p| format!("{:08x}", p.value()))
                    .unwrap_or_else(|| "unknown".to_string())
            }
            _ => "unknown".to_string(),
        };

        Ok(TpmInfo {
            manufacturer,
            is_software: matches!(self.config.interface, TpmInterface::Socket { .. }),
        })
    }

    /// Perform self-test
    pub fn self_test(&mut self, full: bool) -> Result<bool, TpmError> {
        self.context
            .execute_without_session(|ctx| {
                if full {
                    ctx.self_test(tss_esapi::structures::Yes)
                } else {
                    ctx.self_test(tss_esapi::structures::No)
                }
            })
            .map_err(|e| TpmError::OperationFailed(format!("Self-test failed: {}", e)))?;

        Ok(true)
    }
}

/// TPM information
#[derive(Debug, Clone)]
pub struct TpmInfo {
    pub manufacturer: String,
    pub is_software: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TpmConfig::default();
        assert!(matches!(config.interface, TpmInterface::Socket { .. }));
    }

    #[test]
    fn test_config_socket() {
        let config = TpmConfig::socket("127.0.0.1", 2321);
        match config.interface {
            TpmInterface::Socket { host, port } => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, 2321);
            }
            _ => panic!("Expected socket interface"),
        }
    }

    #[test]
    fn test_config_device() {
        let config = TpmConfig::device("/dev/tpm0");
        match config.interface {
            TpmInterface::Device { path } => {
                assert_eq!(path, "/dev/tpm0");
            }
            _ => panic!("Expected device interface"),
        }
    }
}
